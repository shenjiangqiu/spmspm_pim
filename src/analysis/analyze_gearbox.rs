//! this module is used to analyze the gearbox
//! # WARNING:
//!
//! !!! this module is derived from analyze_split_spmm.rs and the code and ***doc*** might not be accurate
use itertools::Itertools;
use rayon::iter::IndexedParallelIterator;
use std::{
    collections::BTreeSet,
    fmt::{Debug, Display},
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

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct GearboxConfig {
    pub topk: f32,
    pub stacks: usize,
    pub layers: usize,
}
struct SubArray {
    row_open: Option<usize>,
    cycle: usize,
}

struct Ring {
    cycle: usize,
    traffic: usize,
}
struct Tsv {
    cycle: usize,
    traffic: usize,
}

struct Hardware {
    sub_array: Vec<SubArray>,
    ring: Vec<Ring>,
    tsv: Vec<Tsv>,
}

struct GearboxSim<'a> {
    ele_per_partition: usize,
    num_partitions: usize,
    evil_col_ids: BTreeSet<usize>,
    evil_row_ids: BTreeSet<usize>,
    matrix_b: CsMat<Pattern>,
    config: &'a GearboxConfig,
    hardware: Hardware,
}
impl<'a> GearboxSim<'a> {
    fn new(
        num_partitions: usize,
        evil_col_ids: impl IntoIterator<Item = usize>,
        evil_row_ids: impl IntoIterator<Item = usize>,
        matrix_b: CsMat<Pattern>,
        config: &'a GearboxConfig,
    ) -> Self {
        debug!(num_partitions, "new gearbox sim");
        let num_rows = matrix_b.rows();
        let ele_per_partition = num_rows / num_partitions;
        assert!(ele_per_partition > 0);
        GearboxSim {
            ele_per_partition,
            num_partitions,
            evil_col_ids: evil_col_ids.into_iter().collect(),
            evil_row_ids: evil_row_ids.into_iter().collect(),
            matrix_b,
            config,
        }
    }
    fn run(&mut self, input_mat: &CsMat<Pattern>) {
        debug!("run gearbox sim");
        let evil_rows = self.evil_row_ids.len();
        let evil_cols = self.evil_col_ids.len();
        debug!(?self.ele_per_partition, ?self.num_partitions, ?evil_rows, ?evil_cols, "run gearbox sim");
        debug!(?self.evil_row_ids, ?self.evil_col_ids, "run gearbox sim");

        todo!()
    }
    fn report(&self) -> SingleResult {
        todo!()
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
        &config.gearbox_config,
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
