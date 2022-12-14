//! this module is used to analyze the nnz

use rayon::prelude::*;
use std::{
    fmt::{Debug, Display},
    ops::Add,
};

use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMat, CsVec, CsVecView};
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
            let new_vecs: Vec<_> = matrix_a
                .outer_iterator()
                .par_bridge()
                .map(|row| {
                    let mut temp_vec = CsVec::new(matrix_b.inner_dims(), vec![], vec![]);
                    for matrix_a_row_idx in row.indices() {
                        let matrix_b_row = matrix_b.outer_view(*matrix_a_row_idx).unwrap();
                        temp_vec = sparse_add(temp_vec.view(), matrix_b_row);
                    }
                    temp_vec.nnz()
                })
                .collect();
            // get the nnz statistics
            let mean = new_vecs.par_iter().sum::<usize>() as f64 / new_vecs.len() as f64;
            let &min = new_vecs.par_iter().min().unwrap();
            let &max = new_vecs.par_iter().max().unwrap();
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

/// add two vector and return a new vector(sparse)
/// # Example
/// ```
/// use spmspm_pim::analysis::analyze_nnz_spmm;
/// use sprs::{CsVec, CsVecView};
/// let v1 = CsVec::new(5, vec![0, 2, 4], vec![1,1,1]);
/// let v2 = CsVec::new(5, vec![1, 3, 4], vec![1,1,1]);
/// let v3 = analyze_nnz_spmm::sparse_add(v1.view(), v2.view());
/// assert_eq!(v3, CsVec::new(5, vec![0, 1, 2, 3, 4], vec![1,1,1,1,2]));
/// ```
pub fn sparse_add<T>(v1: CsVecView<T>, v2: CsVecView<T>) -> CsVec<T>
where
    T: Add<Output = T> + Copy,
{
    assert!(v1.dim() == v2.dim());
    let mut v1_iter = v1.iter();
    let mut v2_iter = v2.iter();
    let mut v1_next = v1_iter.next();
    let mut v2_next = v2_iter.next();
    let mut result = CsVec::empty(v1.dim());
    while v1_next.is_some() || v2_next.is_some() {
        match (v1_next, v2_next) {
            (Some((i1, v1)), Some((i2, v2))) => match i1.cmp(&i2) {
                std::cmp::Ordering::Less => {
                    result.append(i1, *v1);
                    v1_next = v1_iter.next();
                }
                std::cmp::Ordering::Equal => {
                    result.append(i1, *v1 + *v2);
                    v1_next = v1_iter.next();
                    v2_next = v2_iter.next();
                }
                std::cmp::Ordering::Greater => {
                    result.append(i2, *v2);
                    v2_next = v2_iter.next();
                }
            },
            (Some((i1, v1)), None) => {
                result.append(i1, *v1);
                v1_next = v1_iter.next();
            }
            (None, Some((i2, v2))) => {
                result.append(i2, *v2);
                v2_next = v2_iter.next();
            }
            (None, None) => unreachable!(),
        }
    }
    result
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

    #[test]
    fn test_vec_add() {
        init_logger_debug();
        let cs_vec1 = CsVec::new(100, vec![1, 2, 3], vec![Pattern; 3]);
        let cs_vec2 = CsVec::new(100, vec![1, 3, 4], vec![Pattern; 3]);
        let result = sparse_add(cs_vec1.view(), cs_vec2.view());
        debug!(?result);
    }
}
