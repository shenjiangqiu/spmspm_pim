//! this module is used to analyze the gearbox
//! # WARNING:
//!
//! !!! this module is derived from analyze_split_spmm.rs and the code and ***doc*** might not be accurate
use hashbrown::HashSet;
use itertools::Itertools;
use rayon::iter::IndexedParallelIterator;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Debug, Display},
    ops::Deref,
};
use tracing_subscriber::field::debug;

use crate::{
    analysis::split::{split_matrix_by_col, split_matrix_by_row, NnzStats},
    pim::{
        config::Config,
        level::{ddr4, LevelTrait},
    },
};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMat, CsVec, TriMat};
use tracing::{debug, info};

/// the statistics of a single graph
#[derive(Serialize)]
pub struct SingleResult {
    /// the name of the graph
    pub name: String,
    /// the nnz statistics of the graph
    pub nnz_stats_a: NnzStats,
    pub nnz_stats_b: NnzStats,
    /// the cycle and other stats for a graph
    pub graph_result: Vec<SeqResult>,
}
#[derive(Serialize)]
/// the statistics of all graphs
pub struct SplitAnalyzeResult {
    /// the statistics of all graphs
    pub results: Vec<SingleResult>,
}

impl SplitAnalyzeResult {
    /// print out all the results
    pub fn show_results(&self) {
        for result in &self.results {
            println!("---------------------------");
            println!("\n\nname -------: {}", result.name);
            println!("nnz_stats_a: {:?}", result.nnz_stats_a);
            println!("nnz_stats_b: {:?}", result.nnz_stats_b);
            for SeqResult {
                cycle,
                name: _,
                compute_cycle,
                row_open,
                temp_result_read,
                final_result_write,
                matrix_b_read,
                row_open_bytes,
                used_bytes,
                input_read_bytes,
                input_read_times,
            } in &result.graph_result
            {
                println!("cycle: {}", cycle);
                println!("comp_cycle: {}", compute_cycle);
                println!("row_open: {}", row_open);
                println!("temp_result_read: {}", temp_result_read);
                println!("final_result_write: {}", final_result_write);
                println!("matrix_b_read: {}", matrix_b_read);
                println!("row_open_bytes: {}", row_open_bytes);
                println!("used_bytes: {}\n", used_bytes);
                println!("input_read_bytes: {}", input_read_bytes);
                println!("input_read_times: {}\n", input_read_times);
            }
        }
    }
}

impl Debug for SplitAnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for SplitAnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for result in &self.results {
            writeln!(f, "name: {}", result.name)?;
            writeln!(f, "nnz_stats_a: {:?}", result.nnz_stats_a)?;
            writeln!(f, "nnz_stats_b: {:?}", result.nnz_stats_b)?;
        }
        Ok(())
    }
}

/// analyze the split spmm
pub(crate) fn analyze_gearbox(config: &Config) -> SplitAnalyzeResult {
    match config.dram_type {
        crate::pim::config::DramType::DDR3 => todo!(),
        crate::pim::config::DramType::DDR4 => {
            let total_size = ddr4::Storage::new(
                config.channels.num,
                config.ranks.num,
                config.chips.num,
                config.bank_groups.num,
                config.banks.num,
                config.subarrays,
                config.rows,
                config.columns,
            );
            analyze_gearbox_inner::<ddr4::Level>(config, &total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct GearboxConfig {
    pub topk: f32,
    pub stacks: usize,
    pub layers: usize,
}
#[derive(Clone)]
struct SubArray {
    row_open: Option<usize>,
    cycle: usize,
    local_read_rows: Vec<(PhysicRowId, usize)>,
    local_write_rows: Vec<(PhysicRowId, usize)>,
}

impl SubArray {
    fn new() -> Self {
        Self {
            row_open: None,
            cycle: 0,
            local_read_rows: Vec::new(),
            local_write_rows: Vec::new(),
        }
    }
    fn add_task(&mut self, local_read: PhysicRowId, local_write: PhysicRowId) {
        match self.local_read_rows.last() {
            Some((last_read, _)) if *last_read == local_read => {
                self.local_read_rows.last_mut().unwrap().1 += 1;
            }
            _ => {
                self.local_read_rows.push((local_read, 1));
            }
        }
        match self.local_write_rows.last() {
            Some((last_write, _)) if *last_write == local_write => {
                self.local_write_rows.last_mut().unwrap().1 += 1;
            }
            _ => {
                self.local_write_rows.push((local_write, 1));
            }
        }
    }
}

struct RingConfig {}

#[derive(Clone)]
struct Ring {
    cycle: usize,
    traffic: usize,
}
impl Ring {
    fn new() -> Self {
        Self {
            cycle: 0,
            traffic: 0,
        }
    }
    fn add_task(&mut self, source: BankId, target: BankId) {
        self.traffic += 1;
        todo!()
    }
}

struct TsvConfig {}
#[derive(Clone)]
struct Tsv {
    cycle: usize,
    traffic: usize,
}
impl Tsv {
    fn new() -> Self {
        Self {
            cycle: 0,
            traffic: 0,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct LogicRowId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct LogicColId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct PhysicRowId(usize);

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct SubarrayId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct BankId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct RingId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct TsvId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct LayerId(usize);
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
struct StackId(usize);

struct Hardware {
    sub_array: Vec<SubArray>,
    ring: Vec<Ring>,
    tsv: Vec<Tsv>,
    config: Config,
    /// the dimension of dense matrix in one subarray
    dense_dim: usize,
}

impl Hardware {
    fn new(
        num_subarray: usize,
        num_rings: usize,
        num_tsvs: usize,
        dense_dim: usize,
        config: Config,
    ) -> Self {
        Self {
            sub_array: vec![SubArray::new(); num_subarray],
            ring: vec![Ring::new(); num_rings],
            tsv: vec![Tsv::new(); num_tsvs],
            dense_dim,
            config,
        }
    }
    fn distribute_evil_row(
        &mut self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_ids: impl IntoIterator<Item = LogicColId>,
    ) {
        // evil row will always be local
        for col_id in col_ids {
            self.distribute_local_evil_row(target_row_id, mat_b_row_id, col_id);
        }
    }
    /// the total evil cols belongs to this target
    fn distribute_evil_col(
        &mut self,
        target_row_id: LogicRowId,
        row_id_col_id: impl IntoIterator<Item = (LogicRowId, LogicColId)>,
    ) {
        // write to local buffer then write to remote
        // step 1: write to local buffer
        // should be ignored
        // step 2: write to remote once the whole column is finished

        let mut remote_tasks = BTreeMap::new();
        let mut dispatcher_id = None;
        for (mat_b_row_id, col_id) in row_id_col_id {
            let partition_id = self.get_partition_id_row(target_row_id);
            let target_partition_id = self.get_partition_id_col(col_id);
            if partition_id == target_partition_id {
                self.distribute_local(target_row_id, mat_b_row_id, col_id);
            } else {
                // record the remote tasks
                *remote_tasks.entry(col_id).or_insert(0usize) += 1;
                match dispatcher_id {
                    Some(_id) => {}
                    _ => {
                        dispatcher_id = Some(partition_id);
                    }
                }
            }
        }
        // send the remote to ring, tsv and remote subarray
        for entry in remote_tasks {
            let col_id = entry.0;
            // let count = entry.1;
            self.distribute_remote(target_row_id, dispatcher_id.unwrap(), col_id);
        }
    }

    fn distribute_local_evil_row(
        &mut self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.get_partition_id_col(col_id);
        let local_read = self.get_row_id_evil(mat_b_row_id, col_id);
        let local_write = self.get_row_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0].add_task(local_read, local_write);
    }

    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }
    fn get_dispatcher_id(&self, sub_array_id: SubarrayId) -> SubarrayId {
        todo!()
    }
    fn distribute_local(
        &mut self,
        target_row_id: LogicRowId,
        mat_b_row_id: LogicRowId,
        col_id: LogicColId,
    ) {
        // just need to write to the local dense result
        let partition_id = self.get_partition_id_row(mat_b_row_id);
        let local_read = self.get_row_id(mat_b_row_id, col_id);
        let local_write = self.get_row_id_dense(target_row_id, col_id);
        self.sub_array[partition_id.0].add_task(local_read, local_write);
    }

    fn bank_id_from_subarray_id(&self, subarray_id: SubarrayId) -> BankId {
        todo!()
    }
    fn distribute_remote(
        &mut self,
        target_row_id: LogicRowId,
        dispatcher_id: SubarrayId,
        col_id: LogicColId,
    ) {
        // first write to the rings
        let source_layer = self.ring_id(self.bank_id_from_subarray_id(dispatcher_id));
        let target_partition_id = self.get_partition_id_col(col_id);
        let target_layer = self.ring_id(self.bank_id_from_subarray_id(target_partition_id));
        if source_layer == target_layer {
            // no need to write to the csv
            let source = self.bank_id_from_subarray_id(dispatcher_id);

            let target = self.bank_id_from_subarray_id(target_partition_id);
            self.ring[source_layer.0].add_task(source, target);
        } else {
            // write to source ring
            let source_layer = self.ring_id(self.bank_id_from_subarray_id(dispatcher_id));
            // let source_tsv_id = self.ts
            todo!()
        }
    }
    /// from bank id to ring id
    fn ring_id(&self, partition_id: BankId) -> RingId {
        todo!()
    }

    fn get_row_id(&self, mat_b_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }
    fn get_row_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        PhysicRowId((target_row_id.0 * self.dense_dim + col_id.0) / 256)
    }

    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId {
        todo!()
    }
    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId {
        todo!()
    }
    /// reduce the result and return the result
    fn report(&self) -> SingleResult {
        todo!()
    }
}

struct GearboxSim {
    ele_per_partition: usize,
    num_partitions: usize,
    /// the id of evil col: this col will  have a backup copy in each partition
    evil_col_ids: HashSet<usize>,
    /// the id of evil row: the evil row will be partitioned into each components,there are no remote access needed.
    evil_row_ids: HashSet<usize>,
    matrix_b: CsMat<Pattern>,
    hardware: Hardware,
}
impl GearboxSim {
    fn new(
        num_partitions: usize,
        evil_col_ids: impl IntoIterator<Item = usize>,
        evil_row_ids: impl IntoIterator<Item = usize>,
        matrix_b: CsMat<Pattern>,
        config: &Config,
    ) -> Self {
        debug!(num_partitions, "new gearbox sim");
        let num_rows = matrix_b.rows();
        let mut ele_per_partition = num_rows / num_partitions;
        if ele_per_partition == 0 {
            ele_per_partition = 1;
        }
        assert!(ele_per_partition > 0);
        let num_rings = config.gearbox_config.stacks * config.gearbox_config.layers;
        let mut dense_dim = matrix_b.cols() / num_partitions;
        if dense_dim == 0 {
            dense_dim = 1;
        }
        GearboxSim {
            ele_per_partition,
            num_partitions,
            evil_col_ids: evil_col_ids.into_iter().collect(),
            evil_row_ids: evil_row_ids.into_iter().collect(),
            matrix_b,
            hardware: Hardware::new(
                num_partitions,
                num_rings,
                num_rings,
                dense_dim,
                config.clone(),
            ),
        }
    }

    /// distribute the task to components
    fn run(&mut self, input_mat: &CsMat<Pattern>) {
        debug!("run gearbox sim");
        let evil_rows = self.evil_row_ids.len();
        let evil_cols = self.evil_col_ids.len();
        debug!(?self.ele_per_partition, ?self.num_partitions, ?evil_rows, ?evil_cols, "run gearbox sim");
        debug!(?self.evil_row_ids, ?self.evil_col_ids, "run gearbox sim");
        // distribute the task to components
        for (_, (target_id, mat_b_row_id)) in input_mat {
            if self.evil_row_ids.contains(&mat_b_row_id) {
                // the row is evil, no need to access remote
                self.hardware.distribute_evil_row(
                    LogicRowId(target_id),
                    LogicRowId(mat_b_row_id),
                    self.matrix_b
                        .outer_view(mat_b_row_id)
                        .unwrap()
                        .indices()
                        .iter()
                        .map(|&x| LogicColId(x)),
                );
            } else {
                let mut evil_col_row_id_col_id = vec![];
                // the row is not evil, need to access remote
                for col in self.matrix_b.outer_view(mat_b_row_id).unwrap().indices() {
                    if self.evil_col_ids.contains(col) {
                        // the col is evil, no need to access remote
                        // self.hardware
                        //     .distribute_evil_col(target_id, mat_b_row_id, *col);
                        evil_col_row_id_col_id.push((LogicRowId(mat_b_row_id), LogicColId(*col)));
                    } else {
                        // the col is not evil, need to access remote
                        let target_partition = self.get_partition_id_col(LogicColId(*col));
                        let source_partition = self.get_partition_id_row(LogicRowId(mat_b_row_id));
                        if target_partition == source_partition {
                            self.hardware.distribute_local(
                                LogicRowId(target_id),
                                LogicRowId(mat_b_row_id),
                                LogicColId(*col),
                            );
                        } else {
                            // the col is in different partition, need to access remote
                            self.hardware.distribute_remote(
                                LogicRowId(target_id),
                                self.hardware.get_dispatcher_id(
                                    self.hardware.get_partition_id_row(LogicRowId(mat_b_row_id)),
                                ),
                                LogicColId(*col),
                            );
                        }
                    }
                }
                self.hardware
                    .distribute_evil_col(LogicRowId(target_id), evil_col_row_id_col_id);
            }
        }
    }

    /// reduce the result and return the result
    fn report(&self) -> SingleResult {
        self.hardware.report()
    }

    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId {
        self.hardware.get_partition_id_row(row_id)
    }
    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId {
        self.hardware.get_partition_id_col(col_id)
    }
}
fn compute_gearbox(config: &Config, path: &str) -> SingleResult {
    let partitions = config.channels.num
        * config.ranks.num
        * config.chips.num
        * config.bank_groups.num
        * config.banks.num
        * config.subarrays;
    let matrix_a: TriMat<Pattern> = sprs::io::read_matrix_market(path).unwrap();
    let (matrix_a, matrix_b): (CsMat<Pattern>, CsMat<Pattern>) =
        (matrix_a.to_csr(), matrix_a.transpose_view().to_csr());
    let mat_b_rows = matrix_b.rows();
    let mat_b_cols = matrix_b.cols();
    let mut mat_b_row_ids = (0..mat_b_rows)
        .zip(matrix_b.outer_iterator().map(|row| row.nnz()))
        .collect_vec();
    let mut mat_b_col_ids = (0..mat_b_cols)
        .zip(matrix_a.outer_iterator().map(|row| row.nnz()))
        .collect_vec();
    mat_b_row_ids.sort_by_key(|(_index, nnz)| *nnz);
    mat_b_col_ids.sort_by_key(|(_index, nnz)| *nnz);
    let top_rows = (mat_b_col_ids.len() as f32 * config.gearbox_config.topk) as usize;
    assert!(top_rows > 0);

    let top_cols = (mat_b_row_ids.len() as f32 * config.gearbox_config.topk) as usize;
    assert!(top_cols > 0);
    let mut gearbox = GearboxSim::new(
        partitions,
        mat_b_col_ids.iter().take(top_cols).map(|(idx, _)| *idx),
        mat_b_row_ids.iter().take(top_rows).map(|(idx, _)| *idx),
        matrix_b,
        config,
    );

    gearbox.run(&matrix_a);
    gearbox.report()
}
fn analyze_gearbox_inner<LevelType: LevelTrait>(
    config: &Config,
    _total_size: &LevelType::Storage,
) -> SplitAnalyzeResult
where
    LevelType::Storage: Debug + Sync,
    LevelType::Mapping: Debug,
{
    let total_graphs = config.graph_path.len();
    let results = config
        .graph_path
        .iter()
        .enumerate()
        .map(|(index, path)| {
            info!("analyzing graph {}/{}", index + 1, total_graphs);
            compute_gearbox(config, path)
        })
        .collect_vec();

    SplitAnalyzeResult { results }
}

/// the stat result of the seq spmm
#[derive(Debug, Serialize)]
pub struct SeqResult {
    /// the cycles
    pub cycle: u64,
    /// the graph name
    pub name: String,
    /// compute cycles
    pub compute_cycle: u64,
    /// the time spent on row_open:
    pub row_open: u64,
    /// the time spent on temp_result_read
    pub temp_result_read: u64,
    /// the time spent on final_result_write
    pub final_result_write: u64,
    /// the time spent on matrix_b_read
    pub matrix_b_read: u64,
    /// the number of bytes that row opens
    pub row_open_bytes: usize,
    /// the number of bytes that really used
    pub used_bytes: usize,
    /// total input read bytes
    pub input_read_bytes: usize,
    /// total input read times
    pub input_read_times: usize,
}

#[derive(Default, Debug)]
struct SubarrayStatus {
    opened_row: Option<usize>,
    last_read_col: usize,
}

impl SubarrayStatus {
    /// create a new subarray status
    #[allow(unused)]
    fn new() -> Self {
        Default::default()
    }
    /// open a row
    ///
    /// # Arguments
    /// - `start`: the tuple of (row, col)
    /// - `length`: the tuple of (row, col)
    /// - `activate_cycle`: the activate cycle
    /// - `precharge_cycle`: the precharge cycle
    /// - `columns`: the cols of a row
    /// # Return
    /// the tuple (first_row_cycle, remaining_cycle,row_activated)
    fn open_row(
        &mut self,
        start: (usize, usize),
        length: usize,
        activate_cycle: usize,
        precharge_cycle: usize,
        columns: usize,
    ) -> (usize, usize, usize) {
        if length == 0 {
            return (0, 0, 0);
        }
        let mut total_rows_activated = 0;
        let first_row_cycle = match self.opened_row {
            Some(row) => {
                if row == start.0 {
                    0
                } else {
                    total_rows_activated += 1;
                    activate_cycle + precharge_cycle
                }
            }
            None => {
                total_rows_activated += 1;
                activate_cycle
            }
        };
        // all remaining rows should be precharged and activated
        let final_col = start.1 + length;
        let remaining_rows = (final_col - 1) / columns;
        let final_row = start.0 + remaining_rows;
        let remaining_cycle = remaining_rows * (activate_cycle + precharge_cycle);
        self.opened_row = Some(final_row);
        self.last_read_col = (final_col - 1) % columns;
        (
            first_row_cycle,
            remaining_cycle,
            total_rows_activated + remaining_rows,
        )
    }
}

#[cfg(test)]
mod tests {
    use tracing::debug;

    use crate::{
        init_logger_debug,
        pim::config::{Config, LevelConfig},
    };

    use super::*;

    #[test]
    fn test_split_spmm() {
        init_logger_debug();
        let config = Config {
            channels: LevelConfig {
                num: 1,
                ..Default::default()
            },
            ranks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            chips: LevelConfig {
                num: 1,
                ..Default::default()
            },
            bank_groups: LevelConfig {
                num: 1,
                ..Default::default()
            },
            banks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            graph_path: vec!["mtx/test.mtx".to_string()],
            ..Config::from_ddr4_3200(
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
            )
        };
        let result = analyze_gearbox(&config);
        result.show_results();
    }

    #[test]
    fn test_split_spmm_long_vec() {
        init_logger_debug();
        let config = Config {
            channels: LevelConfig {
                num: 1,
                ..Default::default()
            },
            ranks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            chips: LevelConfig {
                num: 1,
                ..Default::default()
            },
            bank_groups: LevelConfig {
                num: 1,
                ..Default::default()
            },
            banks: LevelConfig {
                num: 1,
                ..Default::default()
            },
            graph_path: vec!["mtx/test.mtx".to_string()],
            columns: 8,

            ..Config::from_ddr4_3200(
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
                LevelConfig {
                    num: 1,
                    ..Default::default()
                },
            )
        };

        let result = analyze_gearbox(&config);
        result.show_results();
    }

    #[test]
    fn test_open_row() {
        init_logger_debug();
        let mut subarray = SubarrayStatus::default();
        let result = subarray.open_row((0, 13), 100, 10, 30, 20);
        assert_eq!((10, 200, 0), result);
        assert_eq!(Some(5), subarray.opened_row);
        assert_eq!(12, subarray.last_read_col);
    }
}
