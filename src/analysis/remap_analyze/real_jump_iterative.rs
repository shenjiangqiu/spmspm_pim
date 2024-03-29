//! ## rust module
//! ## Author: Jiangqiu Shen
//! ## Date: 2023-05-11
//! Description: perform the real jump simulation, a real_jump means the one-hot-encoded value should jump to target when current != target
use eyre::Context;
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sprs::{io::MatrixHead, num_kinds::Pattern, CsMatI, CsMatViewI, TriMatI};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::BufWriter,
    iter::repeat,
    mem::size_of,
    time::{Duration, Instant},
};
use tracing::{debug, info};

use crate::{
    algorithms::{bfs::Bfs, page_rank::PageRank, spmm::Spmm, FrontierType, SpmvAlgorithm},
    analysis::{
        remap_analyze::{
            action::{ReduceAction, TotalAction, UpdateAction},
            remote_updator::{
                selective::SelectiveUpdator, sequential::SequentialRemoteUpdator, RemoteUpdator,
            },
            row_cycle::*,
        },
        translate_mapping::{
            self, same_bank::SameBankMapping, weighted::SameBankWeightedMapping, TranslateMapping,
        },
        EVIL_RATE,
    },
    pim::configv2::ConfigV3,
    tools::{self, file_server, FlatInterleaveTrait},
};

use super::IterativeSimulator;
/// ## rust function
/// ## Author: Jiangqiu Shen
/// ## Date: 2023-05-16
/// Description: the temp tasks receive by one subarray
#[derive(Default)]
struct WriteTasks {
    tasks_from_sources: BTreeMap<SubarrayId, Vec<RowIdWordId>>,
}

struct RealJumpSimulator {
    /// the local read of evil row
    evil_row_status: Vec<RowIdWordId>,
    evil_row_cycles: Vec<AllJumpCycles>,
    /// the local read of non evil row
    non_evil_status: Vec<RowIdWordId>,
    non_evil_row_cycles: Vec<AllJumpCycles>,
    /// the remote write
    col_status_remote: Vec<RowIdWordId>,
    col_cycles_remote: Vec<AllJumpCycles>,

    /// the local write
    col_status_local: Vec<RowIdWordId>,
    col_cycles_local: Vec<AllJumpCycles>,
    /// the number of bits of subarrays
    subarray_bits: usize,
    /// the (sending,receiving) status of each bank
    dispatcher_status: Vec<(usize, usize)>,

    /// the cycle of each remap calculation
    remap_cycle: usize,

    /// write tasks
    write_tasks: BTreeMap<SubarrayId, WriteTasks>,
}
struct WriteDenseInfoRemote<'a> {
    source_subarray_id: SubarrayId,
    col_location: &'a RowLocation,
    write_tasks: &'a mut WriteTasks,
}
struct WriteDenseInfoLocal<'a> {
    col_location: &'a RowLocation,
    status: &'a mut RowIdWordId,
    cycle: &'a mut AllJumpCycles,
    remap_cycle: usize,
}

impl RealJumpSimulator {
    pub fn new(
        subarray_size: usize,
        bank_size: usize,
        channel_size: usize,
        remap_cycle: usize,
    ) -> Self {
        assert!(remap_cycle > 0);

        let global_subarray_size = subarray_size * bank_size * channel_size;
        let global_bank_size = bank_size * channel_size;
        let subarray_bits = tools::math::count_to_log(subarray_size);
        Self {
            subarray_bits,
            col_cycles_local: vec![Default::default(); global_subarray_size],
            col_status_local: vec![
                RowIdWordId {
                    row_id: PhysicRowId(0),
                    word_id: WordId(0)
                };
                global_subarray_size
            ],
            col_cycles_remote: vec![Default::default(); global_subarray_size],
            col_status_remote: vec![
                RowIdWordId {
                    row_id: PhysicRowId(0),
                    word_id: WordId(0)
                };
                global_subarray_size
            ],

            dispatcher_status: vec![(0, 0); global_bank_size],
            evil_row_cycles: vec![Default::default(); global_subarray_size],
            evil_row_status: vec![
                RowIdWordId {
                    row_id: PhysicRowId(0),
                    word_id: WordId(0)
                };
                global_subarray_size
            ],
            non_evil_row_cycles: vec![Default::default(); global_subarray_size],
            non_evil_status: vec![
                RowIdWordId {
                    row_id: PhysicRowId(0),
                    word_id: WordId(0)
                };
                global_subarray_size
            ],
            remap_cycle,
            write_tasks: Default::default(),
        }
    }

    fn read_local_evil(&mut self, location: &RowLocation, size: WordId) {
        let current_status = &self.evil_row_status[location.subarray_id.0];

        debug!(
            ?current_status,
            "read localEVIL for subarray{}: {:?}", location.subarray_id.0, location
        );
        // it's the same row
        // self.evil_row_cycles[location.subarray_id.0].update(
        //     self.evil_row_status.get(location.subarray_id.0).unwrap(),
        //     location,
        //     size,
        //     self.remap_cycle,
        // );

        let mut update_action = UpdateAction {
            row_status: self.evil_row_status.get(location.subarray_id.0).unwrap(),
            loc: location,
            size,
            remap_cycle: self.remap_cycle,
        };
        self.evil_row_cycles[location.subarray_id.0].apply_mut(&mut update_action);
        // update the evil row status
        self.evil_row_status[location.subarray_id.0] = location.row_id_word_id.clone();
        let new_status: &RowIdWordId = &self.evil_row_status[location.subarray_id.0];
        debug!(?new_status);
    }
    /// ## rust function
    /// ## Author: Jiangqiu Shen
    /// ## Date: 2023-05-16
    /// Description: write the dense data
    fn write_dense_lazy(write_dense_info: WriteDenseInfoRemote) {
        // fix bug here, instead of simply update the cycle, we should record the tasks first and then reduce it after each round
        let WriteDenseInfoRemote {
            source_subarray_id,
            col_location,
            write_tasks,
            ..
        } = write_dense_info;

        // let mut update_action = UpdateAction {
        //     row_status: status,
        //     loc: col_location,
        //     size: WordId(1),
        //     remap_cycle,
        // };
        // cycle.apply_mut(&mut update_action);
        // *status = col_location.row_id_word_id.clone();

        write_tasks
            .tasks_from_sources
            .entry(source_subarray_id)
            .or_default()
            .push(col_location.row_id_word_id.clone());
    }

    fn write_dense_now(write_dense_info: WriteDenseInfoLocal) {
        let WriteDenseInfoLocal {
            col_location,
            status,
            cycle,
            remap_cycle,
            ..
        } = write_dense_info;
        // fix bug here, instead of simply update the cycle, we should record the tasks first and then reduce it after each round
        debug!(
            ?status,
            "write col for subarray{}: {:?}", col_location.subarray_id.0, col_location
        );
        let mut update_action = UpdateAction {
            row_status: status,
            loc: col_location,
            size: WordId(1),
            remap_cycle,
        };
        cycle.apply_mut(&mut update_action);
        *status = col_location.row_id_word_id.clone();

        debug!(?status);
    }
    fn write_dense_remote(&mut self, source_subarray_id: SubarrayId, col_location: &RowLocation) {
        let write_tasks = self
            .write_tasks
            .entry(col_location.subarray_id)
            .or_insert(Default::default());
        let write_dense_info = WriteDenseInfoRemote {
            source_subarray_id,
            col_location,
            write_tasks,
        };
        Self::write_dense_lazy(write_dense_info);
    }
    fn write_dense_local(&mut self, _source_subarray_id: SubarrayId, col_location: &RowLocation) {
        let current_status = self
            .col_status_local
            .get_mut(col_location.subarray_id.0)
            .unwrap();

        let current_cycle = self
            .col_cycles_local
            .get_mut(col_location.subarray_id.0)
            .unwrap();

        let write_dense_info = WriteDenseInfoLocal {
            col_location,
            status: current_status,
            cycle: current_cycle,
            remap_cycle: self.remap_cycle,
        };
        Self::write_dense_now(write_dense_info);
    }

    fn read_local(&mut self, location: &RowLocation, word_size: WordId) {
        let current_status = &self.non_evil_status[location.subarray_id.0];
        debug!(
            ?current_status,
            "read local for subarray{}: {:?}", location.subarray_id.0, location
        );
        let mut update_action = UpdateAction {
            row_status: current_status,
            loc: location,
            size: word_size,
            remap_cycle: self.remap_cycle,
        };
        self.non_evil_row_cycles[location.subarray_id.0].apply_mut(&mut update_action);
        self.non_evil_status[location.subarray_id.0] = location.row_id_word_id.clone();
        let new_status = &self.non_evil_status[location.subarray_id.0];
        debug!(?new_status);
    }

    fn write_tsv_sending(&mut self, subarray_id: SubarrayId) {
        let bank_id = self.bank_id_from_subarray_id(subarray_id);
        self.dispatcher_status[bank_id].0 += 1;
    }

    fn write_tsv_reading(&mut self, subarray_id: SubarrayId) {
        let bank_id = self.bank_id_from_subarray_id(subarray_id);
        self.dispatcher_status[bank_id].1 += 1;
    }

    fn bank_id_from_subarray_id(&self, subarray_id: SubarrayId) -> usize {
        subarray_id.0 >> self.subarray_bits
    }

    ///[normal, ideal, from_source, my, smart]
    fn update_result(&mut self, result: &mut RealJumpResult) {
        // first reduce the write tasks
        let current_tasks = std::mem::take(&mut self.write_tasks);
        for (target_subarray, tasks) in current_tasks.into_iter() {
            let tasks = tasks.tasks_from_sources;
            // first we need to flat interleave the tasks
            let flat_tasks = tasks.into_values().flat_interleave();
            let remote_dense_cycles = self.col_cycles_remote.get_mut(target_subarray.0).unwrap();

            let remote_dense_status = self.col_status_remote.get_mut(target_subarray.0).unwrap();

            for task in flat_tasks {
                let mut update_action = UpdateAction {
                    row_status: remote_dense_status,
                    loc: &RowLocation {
                        subarray_id: target_subarray,
                        row_id_word_id: task,
                    },
                    size: WordId(1),
                    remap_cycle: self.remap_cycle,
                };
                remote_dense_cycles.apply_mut(&mut update_action);
                *remote_dense_status = task;
            }
        }

        update_row_cycle(&self.col_cycles_local, &mut result.local_dense_col_cycles);
        update_row_cycle(&self.col_cycles_remote, &mut result.remote_dense_col_cycles);

        update_row_cycle(&self.evil_row_cycles, &mut result.evil_row_cycles);
        update_row_cycle(&self.non_evil_row_cycles, &mut result.row_cycles);

        let subarrays = self.non_evil_row_cycles.len() / self.dispatcher_status.len();
        let dispatcher_expand = self
            .dispatcher_status
            .iter()
            .flat_map(|x| repeat(x.0).take(subarrays));

        let local_stage = self
            .col_cycles_local
            .iter()
            .zip(self.non_evil_row_cycles.iter())
            .zip(self.evil_row_cycles.iter())
            .zip(dispatcher_expand)
            .map(|(((a, b), c), d)| (a, b, c, d));

        let local_max = local_stage
            .map(|(local_write, row, evil_row, dispatcher_send)| {
                let mut total_action = TotalAction::default();
                local_write.apply(&mut total_action);
                let local_total = total_action.total;

                let mut total_action = TotalAction::default();
                row.apply(&mut total_action);
                let row_total = total_action.total;

                let mut total_action = TotalAction::default();
                evil_row.apply(&mut total_action);
                let evil_row_total = total_action.total;

                local_total
                    .into_iter()
                    .zip(row_total)
                    .zip(evil_row_total)
                    .map(|((lc, r), evr)| {
                        let total = lc + r + evr;
                        total.max(dispatcher_send)
                    })
                    .collect_vec()
            })
            .reduce(|mut va, vb| {
                va.iter_mut().zip(vb).for_each(|(a, b)| *a = (*a).max(b));
                va
            })
            .unwrap();

        let max_sending_cycle = self.dispatcher_status.iter().map(|x| x.0).max().unwrap();
        let max_receive_cycle = self.dispatcher_status.iter().map(|x| x.1).max().unwrap();
        result.dispatcher_sending_cycle += max_sending_cycle;
        result.dispatcher_reading_cycle += max_receive_cycle;

        assert_eq!(result.real_local_cycle.len(), local_max.len());
        result
            .real_local_cycle
            .iter_mut()
            .zip(local_max)
            .for_each(|(r, l)| {
                *r += l;
            });

        // reset the cycle
        self.col_cycles_local = vec![Default::default(); self.col_cycles_local.len()];
        self.col_cycles_remote = vec![Default::default(); self.col_cycles_remote.len()];

        self.evil_row_cycles = vec![Default::default(); self.evil_row_cycles.len()];
        self.non_evil_row_cycles = vec![Default::default(); self.non_evil_row_cycles.len()];

        self.dispatcher_status = vec![(0, 0); self.dispatcher_status.len()];
    }

    fn process_one_row(
        &mut self,
        target_row: sprs::CsVecBase<&[u32], &[Pattern], Pattern, u32>,
        csr_translated: CsMatViewI<Pattern, u32>,
        mapping: &impl TranslateMapping,
        target_id: usize,
        result: &mut RealJumpResult,
    ) {
        let mut evil_col_handler = EvilColHandler::new();
        // this is a single task
        for &matrix_b_row_id in target_row.indices() {
            self.update_one_column(
                matrix_b_row_id,
                csr_translated,
                mapping,
                target_id,
                &mut evil_col_handler,
            );
        }
        let all_tasks = evil_col_handler.finish();
        for (subarray_id, cols) in all_tasks {
            for col in cols {
                let col_location =
                    mapping.get_dense_location(target_id.into(), col, csr_translated.view());
                if subarray_id == col_location.subarray_id {
                    self.write_dense_local(subarray_id, &col_location);
                } else {
                    // first send to the remote dispacher, ring and tsv,
                    self.write_tsv_sending(subarray_id);
                    self.write_tsv_reading(col_location.subarray_id);
                    self.write_dense_remote(subarray_id, &col_location);
                }
            }
        }
        // after each target_id, update the result and clear current status.
        self.update_result(result);
    }

    fn update_one_column(
        &mut self,
        matrix_b_row_id: u32,
        csr_translated: CsMatViewI<Pattern, u32>,
        mapping: &impl TranslateMapping,
        target_id: usize,
        evil_col_handler: &mut EvilColHandler,
    ) {
        let matrix_b_row_id = LogicRowId::new(matrix_b_row_id as usize);
        let matrix_b_row = csr_translated.outer_view(matrix_b_row_id.0).unwrap();
        if mapping.is_evil(matrix_b_row_id) {
            let evil_location = mapping.get_location_evil(matrix_b_row_id, csr_translated.view());
            for (_subarray_id, row_location, row_vec) in evil_location {
                // send evil tasks to location
                // one nnz is two words, include the index and the data pair!
                let words = row_vec.nnz() * 2;
                self.read_local_evil(&row_location, WordId(words));
                for target_col in row_vec.indices() {
                    let target_col = LogicColId::new(*target_col as usize);
                    let col_location = mapping.get_dense_location(
                        target_id.into(),
                        target_col,
                        csr_translated.view(),
                    );
                    // send write task to subarray
                    self.write_dense_local(_subarray_id, &col_location);
                }
            }
        } else {
            // it's not evil, so read the row
            let location = mapping.get_location(matrix_b_row_id, csr_translated.view());
            // send read task to subarray
            let words = matrix_b_row.nnz() * 2;
            self.read_local(&location, WordId(words));
            // for each column , send write task to subarray
            for &target_col in matrix_b_row.indices() {
                // should handle the evil col
                let dense_location = mapping.get_dense_location(
                    target_id.into(),
                    LogicColId::new(target_col as usize),
                    csr_translated.view(),
                );
                if mapping.is_evil((target_col as usize).into()) {
                    // this is the evil col, should be handled differently
                    evil_col_handler.add_task(location.subarray_id, matrix_b_row_id, target_col);
                } else if location.subarray_id == dense_location.subarray_id {
                    // send write task to subarray
                    self.write_dense_local(location.subarray_id, &dense_location);
                } else {
                    // first send to the remote dispacher, ring and tsv,
                    self.write_tsv_sending(location.subarray_id);
                    self.write_tsv_reading(dense_location.subarray_id);
                    self.write_dense_remote(location.subarray_id, &dense_location);
                    // then send to the subarray
                }
            }
        }
    }

    fn process_one_fullrow(
        &mut self,
        csr_translated: CsMatViewI<Pattern, u32>,
        mapping: &impl TranslateMapping,
        target_id: usize,
        result: &mut RealJumpResult,
    ) {
        let mut evil_col_handler = EvilColHandler::new();
        // this is a single task
        for matrix_b_row_id in 0..csr_translated.rows() as u32 {
            self.update_one_column(
                matrix_b_row_id,
                csr_translated,
                mapping,
                target_id,
                &mut evil_col_handler,
            );
        }
        let all_tasks = evil_col_handler.finish();
        for (subarray_id, cols) in all_tasks {
            for col in cols {
                let col_location =
                    mapping.get_dense_location(target_id.into(), col, csr_translated.view());
                if subarray_id == col_location.subarray_id {
                    self.write_dense_local(subarray_id, &col_location);
                } else {
                    // first send to the remote dispacher, ring and tsv,
                    self.write_tsv_sending(subarray_id);
                    self.write_tsv_reading(col_location.subarray_id);
                    self.write_dense_remote(subarray_id, &col_location);
                }
            }
        }
        // after each target_id, update the result and clear current status.
        self.update_result(result);
    }
}

///[normal, ideal, from_source, my, smart]
/// find the slowest cycle and accumulate it to the final cycle
fn update_row_cycle(
    current_round_cycle: &[AllJumpCycles],
    final_cycle: &mut AllJumpCycles,
) -> [usize; TOTAL_TYPES_COUNT] {
    let mut reduce_action = ReduceAction::default();
    AllJumpCycles::apply_reduce(current_round_cycle, final_cycle, &mut reduce_action);
    reduce_action.total_cycles
}

pub fn run_with_mapping<A: SpmvAlgorithm>(
    mapping: &impl TranslateMapping,
    config: &ConfigV3,
    matrix_csr: CsMatViewI<Pattern, u32>,
    mut algorithm: A,
) -> eyre::Result<RealJumpResult> {
    let remap_cycle = config.remap_cycle;
    info!("remap cycle: {}", remap_cycle);
    let mut simulator = RealJumpSimulator::new(
        config.subarrays,
        config.banks.num,
        config.channels.num,
        remap_cycle,
    );
    info!("start to run simulator");
    simulator.run(mapping, matrix_csr, &mut algorithm)
}
pub fn build_same_bank_mapping(
    config: &ConfigV3,
    matrix_tri: &TriMatI<Pattern, u32>,
    matrix_csr: &CsMatI<Pattern, u32>,
) -> (SameBankMapping, CsMatI<Pattern, u32>) {
    let row_evil_threshold = (matrix_tri.rows() as f32 * EVIL_RATE) as usize;
    translate_mapping::same_bank::SameBankMapping::new(
        config.banks.num,
        config.channels.num,
        config.subarrays,
        row_evil_threshold,
        config.columns,
        matrix_tri,
        matrix_csr,
    )
}
pub fn build_weighted_mapping(
    config: &ConfigV3,
    matrix_tri: &TriMatI<Pattern, u32>,
    matrix_csr: &CsMatI<Pattern, u32>,
) -> (SameBankWeightedMapping, CsMatI<Pattern, u32>) {
    let row_evil_threshold = (matrix_tri.rows() as f32 * EVIL_RATE) as usize;
    translate_mapping::weighted::SameBankWeightedMapping::new(
        config.banks.num,
        config.channels.num,
        config.subarrays,
        row_evil_threshold,
        config.columns,
        matrix_tri,
        matrix_csr,
    )
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AllAlgorithomResults {
    pub bfs: RealJumpResult,
    pub page_rank: RealJumpResult,
    pub spmm: RealJumpResult,
}
pub fn run_simulation(config: ConfigV3) -> eyre::Result<()> {
    info!("start simulation");
    let total_graph_results: Vec<eyre::Result<AllAlgorithomResults>> = config
        .graph_path
        .par_iter()
        .map(|graph| {
            info!("run graph: {}", graph);
            // first allocate the memory quota
            let matrix_head: MatrixHead<Pattern, u32> =
                sprs::io::read_matrix_market_from_bufread_head(&mut file_server::file_reader(
                    graph,
                )?)?;
            let rows = matrix_head.rows;
            let nnz = matrix_head.nnz;

            // allocate he hardware guard, for each subarray, there need at least 128 bytes to save the statistics
            let hardware_size = config.channels.num
                * config.bank_groups.num
                * config.banks.num
                * (size_of::<(usize, usize)>()
                    + (config.subarrays
                        * (size_of::<AllJumpCycles>() * 3 + size_of::<(usize, usize)>() * 3)))
                * 2;
            let csr_matrix_size = rows * size_of::<usize>() + nnz * size_of::<u32>();
            let tri_matrix_size = nnz * size_of::<u32>() * 2;
            let row_evil_threshold = (rows as f32 * EVIL_RATE) as usize;
            let row_evil_threshold = row_evil_threshold.max(1);
            // for each subarray, it should keep a subgraph, the ind size is rows*size_of::<usize>(), the data size is nnz*size_of::<u32>()
            let subarray_matrix_size = 2
                * config.channels.num
                * config.bank_groups.num
                * config.banks.num
                * config.subarrays
                * row_evil_threshold
                * size_of::<usize>()
                + nnz * size_of::<u32>();

            // there will be 4 copy of matrics during initialization
            let mut memory_sections = vec![hardware_size as u64, subarray_matrix_size as u64];
            memory_sections.extend([csr_matrix_size as u64; 2]);
            memory_sections.extend([tri_matrix_size as u64; 2]);
            info!("memory sections: {:?}", memory_sections);
            let total_memory = memory_sections.iter().sum::<u64>();
            let kb = total_memory / 1024;
            let mb = kb / 1024;
            let gb = mb / 1024;
            if gb > 0 {
                info!(
                    "total memory: {} GB, {} MB, for graph:{}",
                    gb,
                    mb % 1024,
                    graph
                );
            } else if mb > 0 {
                info!(
                    "total memory: {} MB, {} KB, for graph:{}",
                    mb,
                    kb % 1024,
                    graph
                );
            } else if kb > 0 {
                info!(
                    "total memory: {} KB {} B  for graph:{}",
                    kb,
                    total_memory % 1024,
                    graph
                );
            } else {
                info!("total memory: {} B  for graph:{}", total_memory, graph);
            }
            let mut matrix_guard = crate::acquire_memory_sections(memory_sections);
            info!("Memory allocation succeed for graph: {}", graph);
            let matrix_tri: TriMatI<Pattern, u32> = sprs::io::read_matrix_market_from_bufread(
                &mut file_server::file_reader(graph)
                    .wrap_err(format!("fail to read path:{}", graph))?,
            )
            .wrap_err(format!("fail to parse mtx format in file {}", graph))?;
            let rows = matrix_tri.rows();
            let cols = matrix_tri.cols();
            assert_eq!(rows, cols);

            let result = match config.mapping {
                crate::pim::configv2::MappingType::SameSubarray => todo!(),
                crate::pim::configv2::MappingType::SameBank => {
                    let (mapping, translated_csr) =
                        translate_mapping::same_bank::SameBankMapping::new(
                            config.banks.num,
                            config.channels.num,
                            config.subarrays,
                            row_evil_threshold,
                            config.columns,
                            &matrix_tri,
                            &matrix_tri.to_csr(),
                        );
                    // after created the mapping, there will be 2 copy of the matrix remained
                    matrix_guard.pop().unwrap();
                    matrix_guard.pop().unwrap();
                    matrix_guard.pop().unwrap();
                    run_all_algorithms(&mapping, &config, translated_csr.view())?
                }
                crate::pim::configv2::MappingType::SameBankWeightedMapping => {
                    let (mapping, translated_csr) =
                        translate_mapping::weighted::SameBankWeightedMapping::new(
                            config.banks.num,
                            config.channels.num,
                            config.subarrays,
                            row_evil_threshold,
                            config.columns,
                            &matrix_tri,
                            &matrix_tri.to_csr(),
                        );
                    // after created the mapping, there will be 2 copy of the matrix remained
                    matrix_guard.pop().unwrap();
                    matrix_guard.pop().unwrap();
                    matrix_guard.pop().unwrap();
                    run_all_algorithms(&mapping, &config, translated_csr.view())?
                }
            };
            // it's automatically dropped, but we need to force drop it here to make sure the matrix drop before the matrix guard
            //free the matrix
            drop(matrix_tri);
            // free the matrix guard
            drop(matrix_guard);
            info!("finish graph: {}", graph);
            Ok(result)
        })
        .collect();
    let total_graph_results = total_graph_results
        .into_iter()
        .map(|r| r.wrap_err("fail to run experiemnt").unwrap())
        .collect_vec();

    // let mut simualtor = RealJumpSimulator;
    // let result = simualtor.run(matrix_tri_translated, filter)?;
    serde_json::to_writer_pretty(
        BufWriter::new(File::create(&config.output_path)?),
        &total_graph_results,
    )?;

    Ok(())
}

pub fn run_all_algorithms(
    mapping: &impl TranslateMapping,
    config: &ConfigV3,
    translated_csr: CsMatViewI<Pattern, u32>,
) -> Result<AllAlgorithomResults, eyre::ErrReport> {
    let bfs = run_with_mapping(
        mapping,
        config,
        translated_csr,
        Bfs::new(translated_csr.view()),
    )?;
    let page_rank = run_with_mapping(mapping, config, translated_csr, PageRank)?;
    let spmm = run_with_mapping(
        mapping,
        config,
        translated_csr,
        Spmm::new(translated_csr.view()),
    )?;
    Ok(AllAlgorithomResults {
        bfs,
        page_rank,
        spmm,
    })
}
#[derive(Default)]
struct EvilColHandler {
    tasks: BTreeMap<SubarrayId, BTreeSet<LogicColId>>,
}
impl EvilColHandler {
    fn new() -> Self {
        Self::default()
    }

    fn add_task(&mut self, subarray_id: SubarrayId, _matrix_b_row_id: LogicRowId, target_col: u32) {
        self.tasks
            .entry(subarray_id)
            .or_default()
            .insert(LogicColId::new(target_col as usize));
    }

    fn finish(self) -> BTreeMap<SubarrayId, BTreeSet<LogicColId>> {
        self.tasks
    }
}
///[normal, ideal, from_source, my, smart]
const NUM_JUMP_CYCLES: usize = AllJumpCyclesTypes::End as usize;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RealJumpResult {
    pub local_dense_col_cycles: AllJumpCycles,
    pub remote_dense_col_cycles: AllJumpCycles,
    pub evil_row_cycles: AllJumpCycles,
    pub row_cycles: AllJumpCycles,
    pub dispatcher_sending_cycle: usize,
    pub dispatcher_reading_cycle: usize,
    // pub real_cycle: [usize; 7],
    pub real_local_cycle: [usize; NUM_JUMP_CYCLES],
}
const MAX_RUN_ROUNDS: usize = 10000;
impl IterativeSimulator for RealJumpSimulator {
    type R = RealJumpResult;
    fn run(
        &mut self,
        mapping: &impl TranslateMapping,
        csr_translated: CsMatViewI<Pattern, u32>,
        algorithm: &mut impl SpmvAlgorithm,
    ) -> eyre::Result<Self::R> {
        let start_time = Instant::now();
        let mut next_print_time = Duration::from_secs(60);
        let mut result = RealJumpResult::default();
        let total_rows = csr_translated.rows();
        let mut target_id = 0;
        while let Some(target_row) = algorithm.next_frontier() {
            if (target_id + 1) % 1000 == 0 {
                let elapsed = start_time.elapsed();
                let remaining =
                    elapsed * (MAX_RUN_ROUNDS as u32 - target_id as u32) / target_id as u32;
                if elapsed > next_print_time {
                    info!(
                        "finish {}/{} rows, elapsed: {:?}, estimated reamining_time: {:?}",
                        target_id,
                        total_rows,
                        humantime::format_duration(elapsed).to_string(),
                        humantime::format_duration(remaining).to_string()
                    );
                    next_print_time += Duration::from_secs(60);
                }
                if target_id + 1 >= MAX_RUN_ROUNDS {
                    break;
                }
            }
            match target_row {
                FrontierType::All => {
                    self.process_one_fullrow(csr_translated.view(), mapping, target_id, &mut result)
                }
                FrontierType::Some(target_row) => self.process_one_row(
                    target_row.view(),
                    csr_translated.view(),
                    mapping,
                    target_id,
                    &mut result,
                ),
            };

            target_id += 1;
        }

        Ok(result)
    }
}
