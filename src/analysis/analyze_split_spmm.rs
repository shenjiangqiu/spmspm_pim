//! this module is used to analyze the split spmm

use std::fmt::Debug;

use itertools::Itertools;
use sprs::{num_kinds::Pattern, CsMat};

use crate::{
    analysis::split::split_matrix,
    pim::{
        config::Config,
        level::{ddr4, LevelTrait},
    },
};

use super::split::NnzStats;

pub struct SingleResult {
    pub name: String,
    pub nnz_stats: NnzStats,
}
pub struct SplitAnalyzeResult {
    pub results: Vec<SingleResult>,
}
impl Debug for SplitAnalyzeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for result in &self.results {
            writeln!(f, "name: {}", result.name)?;
            writeln!(f, "nnz_stats: {:?}", result.nnz_stats)?;
        }
        Ok(())
    }
}

pub fn analyze_split_spmm(config: &Config) -> SplitAnalyzeResult {
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
            analyze_split_spmm_inner::<ddr4::Level>(config, &total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

pub fn analyze_split_spmm_inner<LevelType: LevelTrait>(
    config: &Config,
    _total_size: &LevelType::Storage,
) -> SplitAnalyzeResult {
    let results = config
        .graph_path
        .iter()
        .map(|path| {
            let partitions = config.channels.num
                * config.ranks.num
                * config.chips.num
                * config.bank_groups.num
                * config.banks.num;
            let matrix_a: CsMat<Pattern> = sprs::io::read_matrix_market(path).unwrap().to_csr();
            let totoal_nnz = matrix_a.nnz();
            let matrix_b = matrix_a.transpose_view().to_owned();
            assert!(matrix_b.storage() == sprs::CompressedStorage::CSC);
            let cols = matrix_b.cols();
            // build the split points
            // the ideal start nnz
            let split_points = (0..partitions)
                .map(|i| i * totoal_nnz / partitions)
                .collect::<Vec<usize>>();
            let mut real_split_points = vec![];
            let mut current_nnz = 0;
            let mut graph_out_iter = matrix_b.outer_iterator().map(|row| row.nnz()).enumerate();
            let mut current_col = 0;
            'outer: for i in split_points {
                while current_nnz < i {
                    if let Some((col, nnz)) = graph_out_iter.next() {
                        current_nnz += nnz;
                        current_col = col;
                    } else {
                        break 'outer;
                    }
                }
                real_split_points.push(current_col);
            }
            real_split_points
                .extend(std::iter::repeat(cols).take(partitions - real_split_points.len()));
            drop(graph_out_iter);
            let s_matrix = split_matrix(matrix_b, real_split_points);
            // println!("split matrix {}", s_matrix);
            // average and mean man max nnz for sub matrix
            // println!("sub matrix nnz stats:{:?}", s_matrix.nnz_stats());
            SingleResult {
                name: path.to_string(),
                nnz_stats: s_matrix.nnz_stats(),
            }
        })
        .collect_vec();
    SplitAnalyzeResult { results }
}

#[cfg(test)]
mod tests {

    use crate::pim::config::{Config, LevelConfig};

    use super::analyze_split_spmm;

    #[test]
    fn test_split_spmm() {
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
                num: 4,
                ..Default::default()
            },
            graph_path: vec!["mtx/test_large.mtx".to_string()],
            ..Default::default()
        };
        analyze_split_spmm(&config);
    }
}
