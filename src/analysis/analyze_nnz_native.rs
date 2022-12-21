//! this module is used to analyze the nnz

use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMat};
use tracing::info;

use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
};

use super::split::NnzStats;

/// the statistics of a single graph
#[derive(Serialize, Deserialize)]
pub struct SingleResult {
    /// the name of the graph
    pub name: String,
    /// the nnz statistics of the graph
    pub nnz_stats: NnzStats,
}
#[derive(Serialize, Deserialize)]
/// the statistics of all graphs
pub struct NnzAnalyzeResult {
    /// the statistics of all graphs
    pub results: Vec<SingleResult>,
}

impl NnzAnalyzeResult {
    /// print out all the results
    pub fn show_results(&self) {
        for result in &self.results {
            println!("---------------------------");
            println!("\n\nname -------: {}", result.name);
            println!("nnz_stats: {:?}", result.nnz_stats);
        }
    }
}

impl Debug for NnzAnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for NnzAnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for result in &self.results {
            writeln!(f, "name: {}", result.name)?;
            writeln!(f, "nnz_stats: {:?}", result.nnz_stats)?;
        }
        Ok(())
    }
}

/// analyze the split spmm
pub(crate) fn analyze_nnz_spmm(config: &Config) -> NnzAnalyzeResult {
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
            analyze_nnz_spmm_inner::<ddr4::Level>(config, &total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

fn analyze_nnz_spmm_inner<LevelType: LevelTrait>(
    config: &Config,
    _total_size: &LevelType::Storage,
) -> NnzAnalyzeResult
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

            let matrix_a: CsMat<Pattern> = sprs::io::read_matrix_market(path).unwrap().to_csr();
            let matrix_b = matrix_a.transpose_view().to_csr();
            // perform matrix_a * matrix_b
            let matrix_c = &matrix_a * &matrix_b;
            // get the nnz statistics
            let mean = matrix_c.outer_iterator().map(|v| v.nnz()).sum::<usize>() as f64
                / matrix_c.outer_dims() as f64;
            let min = matrix_c
                .outer_iterator()
                .map(|v| v.nnz())
                .min()
                .unwrap_or(0);
            let max = matrix_c
                .outer_iterator()
                .map(|v| v.nnz())
                .max()
                .unwrap_or(0);
            SingleResult {
                name: path.to_string(),
                nnz_stats: NnzStats { mean, min, max },
            }
        })
        .collect();
    NnzAnalyzeResult { results }
}

/// the stat result of the seq spmm
#[derive(Debug, Serialize, Deserialize)]
pub struct SeqResult {
    /// the cycles
    pub cycle: u64,
    /// the graph name
    pub name: String,
    /// compute cycles
    pub compute_cycle: u64,
    /// the time spent on row_open:
    pub row_open: u64,
    /// row open no overlap
    pub row_open_no_overlap: u64,
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

#[cfg(test)]
mod tests {

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
        let result = analyze_nnz_spmm(&config);
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

        let result = analyze_nnz_spmm(&config);
        result.show_results();
    }
}
