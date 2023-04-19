use eyre::Context;
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sprs::io::MatrixHead;
use sprs::{num_kinds::Pattern, CsMatI, TriMatI};
use std::io::BufWriter;
use std::mem::size_of;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
};

use tracing::{debug, info};

use crate::analysis::{translate_mapping, EVIL_RATE};
use crate::{
    analysis::{
        mapping::{LogicColId, LogicRowId, SubarrayId},
        remap_analyze::Simulator,
        translate_mapping::{RowLocation, TranslateMapping},
    },
    pim::configv2::ConfigV3,
    tools::{self, file_server},
};

use super::jump::{
    FromSourceJumpCycle, IdealJumpCycle, JumpCycle, MyJumpCycle, NormalJumpCycle, SmartJumpCycle,
};

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
struct RowCycle {
    open_cycle: usize,
    normal_jump_cycle: NormalJumpCycle,
    ideal_jump_cycle: IdealJumpCycle,
    from_source_jump_cycle: FromSourceJumpCycle,
    my_jump_cycle: MyJumpCycle,
    smart_jump_cycle: SmartJumpCycle,
}
impl RowCycle {
    pub fn new(calculate_remap_cycle: usize, gap: usize) -> Self {
        Self {
            my_jump_cycle: MyJumpCycle::new(calculate_remap_cycle, gap),
            ..Default::default()
        }
    }
}

///[normal, ideal, from_source, my, smart]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FinalRowCycle {
    pub normal_jump_cycle: (usize, NormalJumpCycle),
    pub ideal_jump_cycle: (usize, IdealJumpCycle),
    pub from_source_jump_cycle: (usize, FromSourceJumpCycle),
    pub my_jump_cycle: (usize, MyJumpCycle),
    pub smart_jump_cycle: (usize, SmartJumpCycle),
}

impl FinalRowCycle {
    pub fn into_split_iter(self) -> SplitIter {
        SplitIter {
            final_row_cycle: self,
            index: 0,
        }
    }
}

pub struct SplitIter {
    final_row_cycle: FinalRowCycle,
    index: usize,
}
pub struct SplitItem {
    pub oepn_row: usize,
    pub one_jump: usize,
    pub muliple_jump: usize,
}
impl Iterator for SplitIter {
    type Item = SplitItem;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => SplitItem {
                oepn_row: self.final_row_cycle.normal_jump_cycle.0,
                one_jump: self.final_row_cycle.normal_jump_cycle.1.jump_one_cycle,
                muliple_jump: self.final_row_cycle.normal_jump_cycle.1.jump_multiple_cycle,
            },
            1 => SplitItem {
                oepn_row: self.final_row_cycle.ideal_jump_cycle.0,
                one_jump: self.final_row_cycle.ideal_jump_cycle.1.total_cycle,
                muliple_jump: 0,
            },
            2 => SplitItem {
                oepn_row: self.final_row_cycle.from_source_jump_cycle.0,
                one_jump: self.final_row_cycle.from_source_jump_cycle.1.jump_one_cycle,
                muliple_jump: self
                    .final_row_cycle
                    .from_source_jump_cycle
                    .1
                    .jump_multiple_cycle,
            },
            3 => SplitItem {
                oepn_row: self.final_row_cycle.my_jump_cycle.0,
                one_jump: self.final_row_cycle.my_jump_cycle.1.one_jump_cycle,
                muliple_jump: self.final_row_cycle.my_jump_cycle.1.multi_jump_cycle,
            },
            4 => SplitItem {
                oepn_row: self.final_row_cycle.smart_jump_cycle.0,
                one_jump: self.final_row_cycle.smart_jump_cycle.1.jump_one_cycle,
                muliple_jump: self.final_row_cycle.smart_jump_cycle.1.jump_multiple_cycle,
            },
            _ => {
                return None;
            }
        };
        self.index += 1;
        Some(result)
    }
}

pub struct FinalRowCycleIter {
    final_row_cycle: FinalRowCycle,
    index: usize,
}
impl IntoIterator for FinalRowCycle {
    type Item = usize;

    type IntoIter = FinalRowCycleIter;

    fn into_iter(self) -> Self::IntoIter {
        FinalRowCycleIter {
            final_row_cycle: self,
            index: 0,
        }
    }
}
impl Iterator for FinalRowCycleIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 5 {
            return None;
        }
        let total_cycle = match self.index {
            0 => {
                self.final_row_cycle.normal_jump_cycle.0
                    + self.final_row_cycle.normal_jump_cycle.1.total()
            }
            1 => {
                self.final_row_cycle.ideal_jump_cycle.0
                    + self.final_row_cycle.ideal_jump_cycle.1.total()
            }
            2 => {
                self.final_row_cycle.from_source_jump_cycle.0
                    + self.final_row_cycle.from_source_jump_cycle.1.total()
            }
            3 => {
                self.final_row_cycle.my_jump_cycle.0 + self.final_row_cycle.my_jump_cycle.1.total()
            }
            4 => {
                self.final_row_cycle.smart_jump_cycle.0
                    + self.final_row_cycle.smart_jump_cycle.1.total()
            }
            _ => return None,
        };
        self.index += 1;
        Some(total_cycle)
    }
}

impl RowCycle {
    fn update(&mut self, evil_row_status: (usize, usize), location: &RowLocation, size: usize) {
        // first update the open cycle
        if evil_row_status.0 == location.row_id.0 {
            // no need to open row
        } else {
            self.open_cycle += 18;
        }
        // then calulate the jump cycle
        self.normal_jump_cycle
            .update(evil_row_status, location, size);
        self.from_source_jump_cycle
            .update(evil_row_status, location, size);
        self.ideal_jump_cycle
            .update(evil_row_status, location, size);
        self.my_jump_cycle.update(evil_row_status, location, size);
        self.smart_jump_cycle
            .update(evil_row_status, location, size);
    }
}
struct RealJumpSimulator {
    evil_row_status: Vec<(usize, usize)>,
    evil_row_cycles: Vec<RowCycle>,
    non_evil_status: Vec<(usize, usize)>,
    non_evil_row_cycles: Vec<RowCycle>,
    col_status: Vec<(usize, usize)>,
    col_cycles: Vec<RowCycle>,
    subarray_bits: usize,
    dispatcher_status: Vec<(usize, usize)>,
}

impl RealJumpSimulator {
    pub fn new(
        subarray_size: usize,
        bank_size: usize,
        channel_size: usize,
        remap_cycle: usize,
        gap: usize,
    ) -> Self {
        let global_subarray_size = subarray_size * bank_size * channel_size;
        let global_bank_size = bank_size * channel_size;
        let subarray_bits = tools::math::count_to_log(subarray_size);
        Self {
            subarray_bits,
            col_cycles: vec![RowCycle::new(remap_cycle, gap); global_subarray_size],
            col_status: vec![(0, 0); global_subarray_size],
            dispatcher_status: vec![(0, 0); global_bank_size],
            evil_row_cycles: vec![RowCycle::new(remap_cycle, gap); global_subarray_size],
            evil_row_status: vec![Default::default(); global_subarray_size],
            non_evil_row_cycles: vec![RowCycle::new(remap_cycle, gap); global_subarray_size],
            non_evil_status: vec![Default::default(); global_subarray_size],
        }
    }

    fn read_local_evil(&mut self, location: &RowLocation, size: usize) {
        let current_status = self.evil_row_status[location.subarray_id.0];

        debug!(
            ?current_status,
            "read localEVIL for subarray{}: {:?}", location.subarray_id.0, location
        );
        // it's the same row
        self.evil_row_cycles[location.subarray_id.0].update(
            self.evil_row_status[location.subarray_id.0],
            location,
            size,
        );
        // update the evil row status
        self.evil_row_status[location.subarray_id.0] = (location.row_id.0, location.col_id.0);
        let new_status = self.evil_row_status[location.subarray_id.0];
        debug!(?new_status);
    }

    fn write_local(
        &mut self,
        _target_id: LogicRowId,
        _target_col: LogicColId,
        col_location: &RowLocation,
    ) {
        let current_status = self.col_status[col_location.subarray_id.0];

        debug!(
            ?current_status,
            "write col for subarray{}: {:?}", col_location.subarray_id.0, col_location
        );
        self.col_cycles[col_location.subarray_id.0].update(current_status, col_location, 1);
        self.col_status[col_location.subarray_id.0] =
            (col_location.row_id.0, col_location.col_id.0);
        let new_status = self.col_status[col_location.subarray_id.0];
        debug!(?new_status);
    }

    fn read_local(&mut self, location: &RowLocation, nnz: usize) {
        let current_status = self.non_evil_status[location.subarray_id.0];
        debug!(
            ?current_status,
            "read local for subarray{}: {:?}", location.subarray_id.0, location
        );
        self.non_evil_row_cycles[location.subarray_id.0].update(current_status, &location, nnz);
        self.non_evil_status[location.subarray_id.0] = (location.row_id.0, location.col_id.0);
        let new_status = self.non_evil_status[location.subarray_id.0];
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

    fn update_result(&mut self, result: &mut RealJumpResult) {
        update_row_cycle(&self.col_cycles, &mut result.col_cycles);
        let evil_max = update_row_cycle(&self.evil_row_cycles, &mut result.evil_row_cycles);
        let non_evil_max = update_row_cycle(&self.non_evil_row_cycles, &mut result.row_cycles);

        let max_sending_cycle = self.dispatcher_status.iter().map(|x| x.0).max().unwrap();
        let max_receive_cycle = self.dispatcher_status.iter().map(|x| x.1).max().unwrap();
        result.dispatcher_sending_cycle += max_sending_cycle;
        result.dispatcher_reading_cycle += max_receive_cycle;

        for (result_cycle, evil_max, non_evil_max) in itertools::izip!(
            result.real_cycle.iter_mut(),
            evil_max.iter(),
            non_evil_max.iter()
        ) {
            debug!(
                "the sending cycle is {}, the evil is {},the non evil is {}",
                max_sending_cycle, evil_max, non_evil_max
            );
            *result_cycle += max_sending_cycle.max(*evil_max + *non_evil_max);
        }

        // reset the cycle
        self.col_cycles = vec![Default::default(); self.col_cycles.len()];
        self.evil_row_cycles = vec![Default::default(); self.evil_row_cycles.len()];
        self.non_evil_row_cycles = vec![Default::default(); self.non_evil_row_cycles.len()];
        self.dispatcher_status = vec![(0, 0); self.dispatcher_status.len()];
    }
}

fn update_jump_cycle<T: JumpCycle>(
    current_round_cycle: &[RowCycle],
    mut specific_jump_cycle: impl FnMut(&RowCycle) -> &T,
    final_cycle: &mut FinalRowCycle,
    mut final_jump: impl FnMut(&mut FinalRowCycle) -> &mut (usize, T),
) -> usize {
    let (open_cycle, normal_jump_cycle) = current_round_cycle
        .iter()
        .map(|x| (x.open_cycle, specific_jump_cycle(x)))
        .max_by_key(|x| x.0 + x.1.total())
        .unwrap();
    let final_jump_cycle = final_jump(final_cycle);
    final_jump_cycle.0 += open_cycle;
    final_jump_cycle.1.add(normal_jump_cycle);
    open_cycle + normal_jump_cycle.total()
}
///[normal, ideal, from_source, my, smart]
fn update_row_cycle(
    current_round_cycle: &[RowCycle],
    final_cycle: &mut FinalRowCycle,
) -> [usize; 5] {
    // first select the max cycle
    //the normal jump cycle
    let normal = update_jump_cycle(
        current_round_cycle,
        |x| &x.normal_jump_cycle,
        final_cycle,
        |x| &mut x.normal_jump_cycle,
    );
    // the ideal jump cycle
    let ideal = update_jump_cycle(
        current_round_cycle,
        |x| &x.ideal_jump_cycle,
        final_cycle,
        |x| &mut x.ideal_jump_cycle,
    );
    // the from source jump cycle
    let from_source = update_jump_cycle(
        current_round_cycle,
        |x| &x.from_source_jump_cycle,
        final_cycle,
        |x| &mut x.from_source_jump_cycle,
    );
    // my jump cycle
    let my = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle,
        final_cycle,
        |x| &mut x.my_jump_cycle,
    );
    let smart = update_jump_cycle(
        current_round_cycle,
        |x| &x.smart_jump_cycle,
        final_cycle,
        |x| &mut x.smart_jump_cycle,
    );
    [normal, ideal, from_source, my, smart]

    //
}

fn run_with_mapping(
    mapping: impl TranslateMapping,
    config: &ConfigV3,
    matrix_tri: &TriMatI<Pattern, u32>,
) -> eyre::Result<RealJumpResult> {
    let remap_cycle = config.remap_cycle;
    let remap_gap = config.remap_gap;
    let mut simulator = RealJumpSimulator::new(
        config.subarrays,
        config.banks.num,
        config.channels.num,
        remap_cycle,
        remap_gap,
    );
    simulator.run(mapping, matrix_tri)
}
pub(crate) fn run_simulation(config: ConfigV3) -> eyre::Result<()> {
    info!("start simulation");
    let total_graph_results: Vec<eyre::Result<RealJumpResult>> = config
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
                        * (size_of::<RowCycle>() * 3 + size_of::<(usize, usize)>() * 3)));
            let matrix_size = rows * size_of::<usize>() + nnz * size_of::<u32>();
            let row_evil_threshold = (rows as f32 * EVIL_RATE) as usize;
            let row_evil_threshold = row_evil_threshold.max(1);
            // for each subarray, it should keep a subgraph, the ind size is rows*size_of::<usize>(), the data size is nnz*size_of::<u32>()
            let subarray_matrix_size = config.channels.num
                * config.bank_groups.num
                * config.banks.num
                * config.subarrays
                * row_evil_threshold
                * size_of::<usize>()
                + nnz * size_of::<u32>();

            // there will be 4 copy of matrics during initialization
            let mut memory_sections = vec![hardware_size, subarray_matrix_size];
            memory_sections.extend([matrix_size; 4]);
            info!("memory sections: {:?}", memory_sections);
            let total_memory = memory_sections.iter().sum::<usize>();
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
            let mut matrix_guard = crate::acquire_memory_sections(&memory_sections);
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
                    let (mapping, matrix_tri) = translate_mapping::same_bank::SameBankMapping::new(
                        config.banks.num,
                        config.channels.num,
                        config.subarrays,
                        row_evil_threshold,
                        config.columns,
                        &matrix_tri,
                    );
                    // after created the mapping, there will be 2 copy of the matrix remained
                    matrix_guard.pop().unwrap();
                    matrix_guard.pop().unwrap();

                    run_with_mapping(mapping, &config, &matrix_tri)?
                    // free the hardware guard here, it's automatically dropped
                }
                crate::pim::configv2::MappingType::SameBankWeightedMapping => {
                    let (mapping, matrix_tri) =
                        translate_mapping::weighted::SameBankWeightedMapping::new(
                            config.banks.num,
                            config.channels.num,
                            config.subarrays,
                            row_evil_threshold,
                            config.columns,
                            &matrix_tri,
                        );
                    // after created the mapping, there will be 2 copy of the matrix remained
                    matrix_guard.pop().unwrap();
                    matrix_guard.pop().unwrap();
                    run_with_mapping(mapping, &config, &matrix_tri)
                        .wrap_err("fail to run the real simulator")?
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

#[derive(Serialize, Deserialize, Debug, Default)]

pub struct RealJumpResult {
    pub col_cycles: FinalRowCycle,
    pub evil_row_cycles: FinalRowCycle,
    pub row_cycles: FinalRowCycle,
    pub dispatcher_sending_cycle: usize,
    pub dispatcher_reading_cycle: usize,
    pub real_cycle: [usize; 5],
}
impl super::Simulator for RealJumpSimulator {
    type R = RealJumpResult;
    fn run(
        &mut self,
        mapping: impl TranslateMapping,
        matrix_tri_translated: &TriMatI<Pattern, u32>,
    ) -> eyre::Result<Self::R> {
        let matrix_csr: CsMatI<Pattern, u32> = matrix_tri_translated.to_csr();
        let mut result = RealJumpResult::default();
        for (target_id, target_row) in matrix_csr.outer_iterator().enumerate() {
            let mut evil_col_handler = EvilColHandler::new();
            // this is a single task
            for &matrix_b_row_id in target_row.indices() {
                let matrix_b_row_id = LogicRowId::new(matrix_b_row_id as usize);
                let matrix_b_row = matrix_csr.outer_view(matrix_b_row_id.0).unwrap();
                if mapping.is_evil(matrix_b_row_id) {
                    let evil_location =
                        mapping.get_location_evil(matrix_b_row_id, matrix_csr.view());
                    for (_subarray_id, row_location, row_vec) in evil_location {
                        // send evil tasks to location

                        let size = row_vec.nnz();
                        self.read_local_evil(&row_location, size);
                        for target_col in row_vec.indices() {
                            let target_col = LogicColId::new(*target_col as usize);
                            let col_location = mapping.get_dense_location(
                                target_id.into(),
                                target_col,
                                matrix_csr.view(),
                            );
                            // send write task to subarray
                            self.write_local(target_id.into(), target_col, &col_location);
                        }
                    }
                } else {
                    // it's not evil, so read the row
                    let location = mapping.get_location(matrix_b_row_id, matrix_csr.view());
                    // send read task to subarray
                    self.read_local(&location, matrix_b_row.nnz());
                    // for each column , send write task to subarray
                    for &target_col in matrix_b_row.indices() {
                        // should handle the evil col
                        let dense_location = mapping.get_dense_location(
                            target_id.into(),
                            LogicColId::new(target_col as usize),
                            matrix_csr.view(),
                        );
                        if mapping.is_evil((target_col as usize).into()) {
                            // this is the evil col, should be handled differently
                            evil_col_handler.add_task(
                                location.subarray_id,
                                matrix_b_row_id,
                                target_col,
                            );
                        } else {
                            if location.subarray_id == dense_location.subarray_id {
                                // send write task to subarray
                                self.write_local(
                                    target_id.into(),
                                    LogicColId(target_col as usize),
                                    &dense_location,
                                );
                            } else {
                                // first send to the remote dispacher, ring and tsv,
                                self.write_tsv_sending(location.subarray_id);
                                self.write_tsv_reading(dense_location.subarray_id);
                                self.write_local(
                                    target_id.into(),
                                    LogicColId(target_col as usize),
                                    &dense_location,
                                );
                                // then send to the subarray
                            }
                        }
                    }
                }
            }
            let all_tasks = evil_col_handler.finish();
            for (_subarray_id, cols) in all_tasks {
                for col in cols {
                    let col_location =
                        mapping.get_dense_location(target_id.into(), col, matrix_csr.view());
                    self.write_local(target_id.into(), col, &col_location);
                }
            }
            // after each target_id, update the result and clear current status.
            self.update_result(&mut result);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use tracing::metadata::LevelFilter;

    use crate::init_logger_stderr;

    use super::*;
    #[test]
    fn test_real_jump() {
        init_logger_stderr(LevelFilter::DEBUG);
        let config = toml::from_str(include_str!("../../../configs/real_jump_test.toml")).unwrap();
        run_simulation(config).unwrap();
    }
}
