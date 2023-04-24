use eyre::Context;
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sprs::io::MatrixHead;
use sprs::{num_kinds::Pattern, CsMatI, TriMatI};
use std::io::BufWriter;
use std::iter::repeat;
use std::mem::size_of;
use std::time::{Duration, Instant};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
};
type AllJumpResult = [usize; 13];
use tracing::{debug, info};

use crate::analysis::translate_mapping::same_bank::SameBankMapping;
use crate::analysis::translate_mapping::weighted::SameBankWeightedMapping;
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
    AddableJumpCycle, FromSourceJumpCycle, IdealJumpCycle, JumpCycle, MyJumpCycle,
    MyJumpNoOverhead, MyJumpOpt, NormalJumpCycle, SmartJumpCycle,
};

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]

pub struct RowCycle {
    pub normal_jump_cycle: NormalJumpCycle,
    pub ideal_jump_cycle: IdealJumpCycle,
    pub from_source_jump_cycle: FromSourceJumpCycle,
    pub my_jump_cycle_16: MyJumpCycle,
    pub my_jump_cycle_32: MyJumpCycle,
    pub my_jump_cycle_64: MyJumpCycle,
    pub my_jump_cycle_16_no_overhead: MyJumpNoOverhead,
    pub my_jump_cycle_32_no_overhead: MyJumpNoOverhead,
    pub my_jump_cycle_64_no_overhead: MyJumpNoOverhead,
    pub my_jump_cycle_16_opt: MyJumpOpt,
    pub my_jump_cycle_32_opt: MyJumpOpt,
    pub my_jump_cycle_64_opt: MyJumpOpt,
    pub smart_jump_cycle: SmartJumpCycle,
}

pub struct RowCycleIterator {
    row_cycle: RowCycle,
    jump_type: JumpTypes,
}
impl IntoIterator for RowCycle {
    type Item = usize;
    type IntoIter = RowCycleIterator;
    fn into_iter(self) -> Self::IntoIter {
        RowCycleIterator {
            row_cycle: self,
            jump_type: JumpTypes::Normal,
        }
    }
}
impl Iterator for RowCycleIterator {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let ret = match self.jump_type {
            JumpTypes::Normal => self.row_cycle.normal_jump_cycle.total(),
            JumpTypes::Ideal => self.row_cycle.ideal_jump_cycle.total(),
            JumpTypes::FromSource => self.row_cycle.from_source_jump_cycle.total(),
            JumpTypes::My16 => self.row_cycle.my_jump_cycle_16.total(),
            JumpTypes::My32 => self.row_cycle.my_jump_cycle_32.total(),
            JumpTypes::My64 => self.row_cycle.my_jump_cycle_64.total(),
            JumpTypes::My16NoOverhead => self.row_cycle.my_jump_cycle_16_no_overhead.total(),
            JumpTypes::My32NoOverhead => self.row_cycle.my_jump_cycle_32_no_overhead.total(),
            JumpTypes::My64NoOverhead => self.row_cycle.my_jump_cycle_64_no_overhead.total(),
            JumpTypes::My16Opt => self.row_cycle.my_jump_cycle_16_opt.total(),
            JumpTypes::My32Opt => self.row_cycle.my_jump_cycle_32_opt.total(),
            JumpTypes::My64Opt => self.row_cycle.my_jump_cycle_64_opt.total(),
            JumpTypes::Smart => self.row_cycle.smart_jump_cycle.total(),
            JumpTypes::End => return None,
        };
        self.jump_type.next();
        Some(ret)
    }
}
impl RowCycle {
    pub fn into_split_iter(self) -> RowCycleSplitIter {
        RowCycleSplitIter {
            row_cycle: self,
            jump_type: JumpTypes::Normal,
        }
    }
}
pub struct RowCycleSplitIter {
    row_cycle: RowCycle,
    jump_type: JumpTypes,
}
impl Iterator for RowCycleSplitIter {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let cycle = match self.jump_type {
            JumpTypes::Normal => (
                self.row_cycle.normal_jump_cycle.get_one_jump(),
                self.row_cycle.normal_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::Ideal => (
                self.row_cycle.ideal_jump_cycle.get_one_jump(),
                self.row_cycle.ideal_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::FromSource => (
                self.row_cycle.from_source_jump_cycle.get_one_jump(),
                self.row_cycle.from_source_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::My16 => (
                self.row_cycle.my_jump_cycle_16.get_one_jump(),
                self.row_cycle.my_jump_cycle_16.get_multi_jump(),
            ),
            JumpTypes::My32 => (
                self.row_cycle.my_jump_cycle_32.get_one_jump(),
                self.row_cycle.my_jump_cycle_32.get_multi_jump(),
            ),
            JumpTypes::My64 => (
                self.row_cycle.my_jump_cycle_64.get_one_jump(),
                self.row_cycle.my_jump_cycle_64.get_multi_jump(),
            ),
            JumpTypes::My16NoOverhead => (
                self.row_cycle.my_jump_cycle_16_no_overhead.get_one_jump(),
                self.row_cycle.my_jump_cycle_16_no_overhead.get_multi_jump(),
            ),
            JumpTypes::My32NoOverhead => (
                self.row_cycle.my_jump_cycle_32_no_overhead.get_one_jump(),
                self.row_cycle.my_jump_cycle_32_no_overhead.get_multi_jump(),
            ),
            JumpTypes::My64NoOverhead => (
                self.row_cycle.my_jump_cycle_64_no_overhead.get_one_jump(),
                self.row_cycle.my_jump_cycle_64_no_overhead.get_multi_jump(),
            ),
            JumpTypes::My16Opt => (
                self.row_cycle.my_jump_cycle_16_opt.get_one_jump(),
                self.row_cycle.my_jump_cycle_16_opt.get_multi_jump(),
            ),
            JumpTypes::My32Opt => (
                self.row_cycle.my_jump_cycle_32_opt.get_one_jump(),
                self.row_cycle.my_jump_cycle_32_opt.get_multi_jump(),
            ),
            JumpTypes::My64Opt => (
                self.row_cycle.my_jump_cycle_64_opt.get_one_jump(),
                self.row_cycle.my_jump_cycle_64_opt.get_multi_jump(),
            ),
            JumpTypes::Smart => (
                self.row_cycle.smart_jump_cycle.get_one_jump(),
                self.row_cycle.smart_jump_cycle.get_multi_jump(),
            ),
            JumpTypes::End => return None,
        };
        self.jump_type.next();
        Some(cycle)
    }
}
enum JumpTypes {
    Normal,
    Ideal,
    FromSource,
    My16,
    My32,
    My64,
    My16NoOverhead,
    My32NoOverhead,
    My64NoOverhead,
    My16Opt,
    My32Opt,
    My64Opt,
    Smart,
    End,
}
impl JumpTypes {
    fn next(&mut self) {
        *self = match self {
            JumpTypes::Normal => JumpTypes::Ideal,
            JumpTypes::Ideal => JumpTypes::FromSource,
            JumpTypes::FromSource => JumpTypes::My16,
            JumpTypes::My16 => JumpTypes::My32,
            JumpTypes::My32 => JumpTypes::My64,
            JumpTypes::My64 => JumpTypes::My16NoOverhead,
            JumpTypes::My16NoOverhead => JumpTypes::My32NoOverhead,
            JumpTypes::My32NoOverhead => JumpTypes::My64NoOverhead,
            JumpTypes::My64NoOverhead => JumpTypes::My16Opt,
            JumpTypes::My16Opt => JumpTypes::My32Opt,
            JumpTypes::My32Opt => JumpTypes::My64Opt,
            JumpTypes::My64Opt => JumpTypes::Smart,
            JumpTypes::Smart => JumpTypes::End,
            JumpTypes::End => JumpTypes::End,
        }
    }
}

impl RowCycle {
    fn update(
        &mut self,
        row_status: &(usize, usize),
        location: &RowLocation,
        size: usize,
        remap_cycle: usize,
    ) {
        // then calulate the jump cycle
        self.normal_jump_cycle.update(row_status, location, size);
        self.from_source_jump_cycle
            .update(row_status, location, size);
        self.ideal_jump_cycle.update(row_status, location, size);
        self.my_jump_cycle_16
            .update(row_status, location, size, remap_cycle, 16);
        self.my_jump_cycle_32
            .update(row_status, location, size, remap_cycle, 32);
        self.my_jump_cycle_64
            .update(row_status, location, size, remap_cycle, 64);

        self.smart_jump_cycle.update(row_status, location, size);
    }
}
struct RealJumpSimulator {
    /// the local read of evil row
    evil_row_status: Vec<(usize, usize)>,
    evil_row_cycles: Vec<RowCycle>,
    /// the local read of non evil row
    non_evil_status: Vec<(usize, usize)>,
    non_evil_row_cycles: Vec<RowCycle>,
    /// the remote write
    col_status_remote: Vec<(usize, usize)>,
    col_cycles_remote: Vec<RowCycle>,
    /// the local write
    col_status_local: Vec<(usize, usize)>,
    col_cycles_local: Vec<RowCycle>,
    /// the number of bits of subarrays
    subarray_bits: usize,
    /// the (sending,receiving) status of each bank
    dispatcher_status: Vec<(usize, usize)>,
    /// the cycle of each remap calculation
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
            col_status_local: vec![(0, 0); global_subarray_size],
            col_cycles_remote: vec![Default::default(); global_subarray_size],
            col_status_remote: vec![(0, 0); global_subarray_size],
            dispatcher_status: vec![(0, 0); global_bank_size],
            evil_row_cycles: vec![Default::default(); global_subarray_size],
            evil_row_status: vec![Default::default(); global_subarray_size],
            non_evil_row_cycles: vec![Default::default(); global_subarray_size],
            non_evil_status: vec![Default::default(); global_subarray_size],
            remap_cycle,
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
            self.evil_row_status.get(location.subarray_id.0).unwrap(),
            location,
            size,
            self.remap_cycle,
        );
        // update the evil row status
        self.evil_row_status[location.subarray_id.0] = (location.row_id.0, location.col_id.0);
        let new_status = self.evil_row_status[location.subarray_id.0];
        debug!(?new_status);
    }

    fn write_dense(
        _target_id: LogicRowId,
        _target_col: LogicColId,
        col_location: &RowLocation,
        status: &mut (usize, usize),
        cycle: &mut RowCycle,
        remap_cycle: usize,
    ) {
        debug!(
            ?status,
            "write col for subarray{}: {:?}", col_location.subarray_id.0, col_location
        );
        cycle.update(status, col_location, 1, remap_cycle);
        *status = (col_location.row_id.0, col_location.col_id.0);
        debug!(?status);
    }
    fn write_dense_remote(
        &mut self,
        target_id: LogicRowId,
        target_col: LogicColId,
        col_location: &RowLocation,
    ) {
        let current_status = self
            .col_status_remote
            .get_mut(col_location.subarray_id.0)
            .unwrap();

        let current_cycle = self
            .col_cycles_remote
            .get_mut(col_location.subarray_id.0)
            .unwrap();

        Self::write_dense(
            target_id,
            target_col,
            col_location,
            current_status,
            current_cycle,
            self.remap_cycle,
        );
    }
    fn write_dense_local(
        &mut self,
        target_id: LogicRowId,
        target_col: LogicColId,
        col_location: &RowLocation,
    ) {
        let current_status = self
            .col_status_local
            .get_mut(col_location.subarray_id.0)
            .unwrap();

        let current_cycle = self
            .col_cycles_local
            .get_mut(col_location.subarray_id.0)
            .unwrap();

        Self::write_dense(
            target_id,
            target_col,
            col_location,
            current_status,
            current_cycle,
            self.remap_cycle,
        );
    }

    fn read_local(&mut self, location: &RowLocation, nnz: usize) {
        let current_status = &self.non_evil_status[location.subarray_id.0];
        debug!(
            ?current_status,
            "read local for subarray{}: {:?}", location.subarray_id.0, location
        );
        self.non_evil_row_cycles[location.subarray_id.0].update(
            current_status,
            &location,
            nnz,
            self.remap_cycle,
        );
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

    ///[normal, ideal, from_source, my, smart]
    fn update_result(&mut self, result: &mut RealJumpResult) {
        update_row_cycle(&self.col_cycles_local, &mut result.local_dense_col_cycles);
        update_row_cycle(&self.col_cycles_remote, &mut result.remote_dense_col_cycles);
        update_row_cycle(&self.evil_row_cycles, &mut result.evil_row_cycles);
        update_row_cycle(&self.non_evil_row_cycles, &mut result.row_cycles);

        let subarrays = self.non_evil_row_cycles.len() / self.dispatcher_status.len();
        let dispatcher_expand = self
            .dispatcher_status
            .iter()
            .map(|x| repeat(x.0).take(subarrays))
            .flatten();

        let local_stage = self
            .col_cycles_local
            .iter()
            .zip(self.non_evil_row_cycles.iter())
            .zip(self.evil_row_cycles.iter())
            .zip(dispatcher_expand)
            .map(|(((a, b), c), d)| (a, b, c, d));

        let local_max = local_stage
            .map(|(local_write, row, evil_row, dispatcher_send)| {
                let local_total = local_write.clone().into_iter().collect_vec();
                let row_total = row.clone().into_iter().collect_vec();
                let evil_row_total = evil_row.clone().into_iter().collect_vec();
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

        assert!(result.real_local_cycle.len() == local_max.len());
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
}

fn update_jump_cycle<T: AddableJumpCycle>(
    current_round_cycle: &[RowCycle],
    mut specific_jump_cycle: impl FnMut(&RowCycle) -> &T,
    final_cycle: &mut RowCycle,
    mut final_jump: impl FnMut(&mut RowCycle) -> &mut T,
) -> usize {
    let normal_jump_cycle = current_round_cycle
        .iter()
        .map(|x| specific_jump_cycle(x))
        .max_by_key(|x| x.total())
        .unwrap();
    let final_jump_cycle = final_jump(final_cycle);
    final_jump_cycle.add(normal_jump_cycle);
    normal_jump_cycle.total()
}
///[normal, ideal, from_source, my, smart]
/// find the slowest cycle and accumulate it to the final cycle
fn update_row_cycle(current_round_cycle: &[RowCycle], final_cycle: &mut RowCycle) -> AllJumpResult {
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
    let my_16 = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_16,
        final_cycle,
        |x| &mut x.my_jump_cycle_16,
    );
    let my_32 = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_32,
        final_cycle,
        |x| &mut x.my_jump_cycle_32,
    );
    let my_64 = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_64,
        final_cycle,
        |x| &mut x.my_jump_cycle_64,
    );

    // my jump cycle
    let my_16_no_overhead = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_16_no_overhead,
        final_cycle,
        |x| &mut x.my_jump_cycle_16_no_overhead,
    );
    let my_32_no_overhead = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_32_no_overhead,
        final_cycle,
        |x| &mut x.my_jump_cycle_32_no_overhead,
    );
    let my_64_no_overhead = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_64_no_overhead,
        final_cycle,
        |x| &mut x.my_jump_cycle_64_no_overhead,
    );

    // my jump cycle
    let my_16_opt = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_16_opt,
        final_cycle,
        |x| &mut x.my_jump_cycle_16_opt,
    );
    let my_32_opt = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_32_opt,
        final_cycle,
        |x| &mut x.my_jump_cycle_32_opt,
    );
    let my_64_opt = update_jump_cycle(
        current_round_cycle,
        |x| &x.my_jump_cycle_64_opt,
        final_cycle,
        |x| &mut x.my_jump_cycle_64_opt,
    );
    let smart = update_jump_cycle(
        current_round_cycle,
        |x| &x.smart_jump_cycle,
        final_cycle,
        |x| &mut x.smart_jump_cycle,
    );

    [
        normal,
        ideal,
        from_source,
        my_16,
        my_32,
        my_64,
        my_16_no_overhead,
        my_32_no_overhead,
        my_64_no_overhead,
        my_16_opt,
        my_32_opt,
        my_64_opt,
        smart,
    ]

    //
}

pub fn run_with_mapping(
    mapping: &impl TranslateMapping,
    config: &ConfigV3,
    matrix_csr: &CsMatI<Pattern, u32>,
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
    simulator.run(mapping, matrix_csr)
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
                        * (size_of::<RowCycle>() * 3 + size_of::<(usize, usize)>() * 3)))
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

                    run_with_mapping(&mapping, &config, &translated_csr)?
                    // free the hardware guard here, it's automatically dropped
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
                    run_with_mapping(&mapping, &config, &translated_csr)
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]

pub struct RealJumpResult {
    pub local_dense_col_cycles: RowCycle,
    pub remote_dense_col_cycles: RowCycle,
    pub evil_row_cycles: RowCycle,
    pub row_cycles: RowCycle,
    pub dispatcher_sending_cycle: usize,
    pub dispatcher_reading_cycle: usize,
    // pub real_cycle: [usize; 7],
    pub real_local_cycle: AllJumpResult,
}
impl super::Simulator for RealJumpSimulator {
    type R = RealJumpResult;
    fn run(
        &mut self,
        mapping: &impl TranslateMapping,
        csr_translated: &CsMatI<Pattern, u32>,
    ) -> eyre::Result<Self::R> {
        let start_time = Instant::now();
        let mut next_print_time = Duration::from_secs(60);
        let mut result = RealJumpResult::default();
        let total_rows = csr_translated.rows();
        for (target_id, target_row) in csr_translated.outer_iterator().enumerate() {
            if (target_id + 1) % 1000 == 0 {
                let elapsed = start_time.elapsed();
                let remaining = elapsed * (total_rows as u32 - target_id as u32) / target_id as u32;
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
                if target_id >= 50000 {
                    break;
                }
            }
            let mut evil_col_handler = EvilColHandler::new();
            // this is a single task
            for &matrix_b_row_id in target_row.indices() {
                let matrix_b_row_id = LogicRowId::new(matrix_b_row_id as usize);
                let matrix_b_row = csr_translated.outer_view(matrix_b_row_id.0).unwrap();
                if mapping.is_evil(matrix_b_row_id) {
                    let evil_location =
                        mapping.get_location_evil(matrix_b_row_id, csr_translated.view());
                    for (_subarray_id, row_location, row_vec) in evil_location {
                        // send evil tasks to location

                        let size = row_vec.nnz();
                        self.read_local_evil(&row_location, size);
                        for target_col in row_vec.indices() {
                            let target_col = LogicColId::new(*target_col as usize);
                            let col_location = mapping.get_dense_location(
                                target_id.into(),
                                target_col,
                                csr_translated.view(),
                            );
                            // send write task to subarray
                            self.write_dense_local(target_id.into(), target_col, &col_location);
                        }
                    }
                } else {
                    // it's not evil, so read the row
                    let location = mapping.get_location(matrix_b_row_id, csr_translated.view());
                    // send read task to subarray
                    self.read_local(&location, matrix_b_row.nnz());
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
                            evil_col_handler.add_task(
                                location.subarray_id,
                                matrix_b_row_id,
                                target_col,
                            );
                        } else {
                            if location.subarray_id == dense_location.subarray_id {
                                // send write task to subarray
                                self.write_dense_local(
                                    target_id.into(),
                                    LogicColId(target_col as usize),
                                    &dense_location,
                                );
                            } else {
                                // first send to the remote dispacher, ring and tsv,
                                self.write_tsv_sending(location.subarray_id);
                                self.write_tsv_reading(dense_location.subarray_id);
                                self.write_dense_remote(
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
            for (subarray_id, cols) in all_tasks {
                for col in cols {
                    let col_location =
                        mapping.get_dense_location(target_id.into(), col, csr_translated.view());
                    if subarray_id == col_location.subarray_id {
                        self.write_dense_local(target_id.into(), col, &col_location);
                    } else {
                        // first send to the remote dispacher, ring and tsv,
                        self.write_tsv_sending(subarray_id);
                        self.write_tsv_reading(col_location.subarray_id);
                        self.write_dense_remote(target_id.into(), col, &col_location);
                    }
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
