//! this module is used to analyze the split spmm

use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
};

use itertools::Itertools;
use sprs::{num_kinds::Pattern, CsMat, CsVec, CsVecView};
use tracing::{info, trace};

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
        Display::fmt(&self, f)
    }
}

impl Display for SplitAnalyzeResult {
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
    total_size: &LevelType::Storage,
) -> SplitAnalyzeResult
where
    LevelType::Storage: Debug,
    LevelType::Mapping: Debug,
{
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
            for (bank_id, single_matrix) in s_matrix.matrix.iter().enumerate() {
                let bank_result = compute_bank_cycle_seq::<LevelType>(
                    config,
                    path.to_string(),
                    single_matrix,
                    &matrix_a,
                    bank_id,
                    total_size,
                );
                info!(?bank_result);
            }
            SingleResult {
                name: path.to_string(),
                nnz_stats: s_matrix.nnz_stats(),
            }
        })
        .collect_vec();
    SplitAnalyzeResult { results }
}

/// the stat result of the seq spmm
#[derive(Debug)]
pub struct SeqResult {
    /// the cycles
    pub cycle: u64,
    /// the graph name
    pub name: String,
}

/// add two vector and return a new vector(sparse)
fn sparse_add(v1: CsVecView<Pattern>, v2: CsVecView<Pattern>) -> CsVec<Pattern> {
    assert!(v1.dim() == v2.dim());
    let mut v1_iter = v1.iter();
    let mut v2_iter = v2.iter();
    let mut v1_next = v1_iter.next();
    let mut v2_next = v2_iter.next();
    let mut result = CsVec::empty(v1.dim());
    while v1_next.is_some() || v2_next.is_some() {
        match (v1_next, v2_next) {
            (Some((i1, _)), Some((i2, _))) => {
                if i1 == i2 {
                    result.append(i1, Pattern);
                    v1_next = v1_iter.next();
                    v2_next = v2_iter.next();
                } else if i1 < i2 {
                    result.append(i1, Pattern);
                    v1_next = v1_iter.next();
                } else {
                    result.append(i2, Pattern);
                    v2_next = v2_iter.next();
                }
            }
            (Some((i1, _)), None) => {
                result.append(i1, Pattern);
                v1_next = v1_iter.next();
            }
            (None, Some((i2, _))) => {
                result.append(i2, Pattern);
                v2_next = v2_iter.next();
            }
            (None, None) => unreachable!(),
        }
    }
    result
}

#[derive(Default, Debug)]
struct SubarrayStatus {
    opened_row: Option<usize>,
    last_read_col: usize,
}

impl SubarrayStatus {
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
    /// - `row_size`: the cols of a row
    /// # Return
    /// the tuple (first_row_cycle, remaining_cycle)
    fn open_row(
        &mut self,
        start: (usize, usize),
        length: usize,
        activate_cycle: usize,
        precharge_cycle: usize,
        row_size: usize,
    ) -> (usize, usize) {
        if length == 0 {
            return (0, 0);
        }
        let first_row_cycle = match self.opened_row {
            Some(row) => {
                if row == start.0 {
                    0
                } else {
                    activate_cycle + precharge_cycle
                }
            }
            None => activate_cycle,
        };
        // all remaining rows should be precharged and activated
        let final_col = start.1 + length;
        let remaining_rows = (final_col - 1) / row_size;
        let final_row = start.0 + remaining_rows;
        let remaining_cycle = remaining_rows * (activate_cycle + precharge_cycle);
        self.opened_row = Some(final_row);
        self.last_read_col = (final_col - 1) % row_size;
        (first_row_cycle, remaining_cycle)
    }
}

/// compute the run cycles for each bank in sequence
///
/// in sequence means that tasks are executed in the order of the input,
/// the bank will read the tasks one by one and read the row of the output and input, and then
///  accumulate the input into the result.
///
/// # Arguments
/// - `config`: the config
/// - `single_matrix`: the matrix b belongs to the bank
/// - `input_matrix`: the input matrix
/// - `bank_id`: the bank id
///
/// # Returns
/// - [`SeqResult`]: the stats
///  
pub fn compute_bank_cycle_seq<LevelType: LevelTrait>(
    config: &Config,
    path: String,
    single_matrix: &CsMat<Pattern>,
    input_matrix: &CsMat<Pattern>,
    bank_id: usize,
    total_size: &LevelType::Storage,
) -> SeqResult
where
    LevelType::Storage: Debug,
    LevelType::Mapping: Debug,
{
    let mut cycle: u64 = 0;
    // first we need to map the matrix to the bank
    // reset to 1 until subarray
    let mappings = LevelType::get_mapping(
        &LevelType::set_one_to_level(total_size, &LevelType::last_level()),
        input_matrix,
    );
    // assume we have the two sub arrays to store the partial result(it's not affecting the cycle accuracy)
    let mut temp_result_subarray = SubarrayStatus::default();
    let mut final_result_subarray = SubarrayStatus::default();
    // a map from subarray to status
    let mut open_row_status: hashbrown::HashMap<usize, SubarrayStatus> = Default::default();
    trace!(?mappings);
    for task in input_matrix.outer_iterator() {
        let mut current_result: CsVec<Pattern> =
            CsVec::new(single_matrix.inner_dims(), vec![], vec![]);
        // mean the result should be write to the temporary result subarray.
        let mut reverse_result = task.nnz() % 2 == 0;
        for (task_id_b, _) in task.iter() {
            // first determine current round temp and final
            let (current_temp, current_final) = if reverse_result {
                (&mut final_result_subarray, &mut temp_result_subarray)
            } else {
                (&mut temp_result_subarray, &mut final_result_subarray)
            };
            reverse_result = !reverse_result;

            let input_row = single_matrix.outer_view(task_id_b).unwrap();
            // the cycle to read temp result(open row)
            let (temp_result1, temp_result2) = current_temp.open_row(
                (0, 0),
                current_result.nnz() * 4,
                config.activate_cycle as usize,
                config.precharge_cycle as usize,
                config.row_size,
            );

            current_result = sparse_add(current_result.view(), input_row);
            // the cycle to write final result(open row)
            let (final_result1, final_result2) = current_final.open_row(
                (0, 0),
                current_result.nnz() * 4,
                config.activate_cycle as usize,
                config.precharge_cycle as usize,
                config.row_size,
            );
            // calculate the cycle:
            // 1. the cycle to calculate the nnz
            cycle += current_result.nnz() as u64;
            // 2. the cycle to open the row of the input matrix
            let input_row_detail = LevelType::get_row_detail(&mappings, task_id_b);
            let path = &input_row_detail.path;
            let subarray_id = LevelType::subarray().get_level_id(path);
            let row_id = LevelType::row().get_level_id(path);
            let col_id = LevelType::col().get_level_id(path);
            let input_subarray = open_row_status.entry(subarray_id).or_default();
            // the cycle to open the row of the input matrix
            let (input_cycle1, input_cycle2) = input_subarray.open_row(
                (row_id, col_id),
                input_row_detail.size,
                config.activate_cycle as usize,
                config.precharge_cycle as usize,
                config.row_size,
            );
            cycle += itertools::max([temp_result1, final_result1, input_cycle1]).unwrap() as u64;
            cycle += [temp_result2, final_result2, input_cycle2]
                .iter()
                .sum::<usize>() as u64;
        }
    }
    SeqResult { cycle, name: path }
}

/// compute the run cycles for each bank in parallel
///
/// # Arguments
/// - `config`: the config
/// - `single_matrix`: the matrix b belongs to the bank
/// - `input_matrix`: the input matrix
/// - `bank_id`: the bank id
pub fn compute_bank_cycle_parallel<LevelType: LevelTrait>(
    config: &Config,
    single_matrix: CsMat<Pattern>,
    input_matrix: &CsMat<Pattern>,
    bank_id: &LevelType::Storage,
) -> u64 {
    todo!()
}

#[cfg(test)]
mod tests {
    use tracing::{debug, info};

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
                num: 4,
                ..Default::default()
            },
            graph_path: vec!["mtx/bcspwr06.mtx".to_string()],
            ..Config::from_ddr4(
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
        let result = analyze_split_spmm(&config);
        info!(?result);
    }

    #[test]
    fn test_open_row() {
        init_logger_debug();
        let mut subarray = SubarrayStatus::default();
        let result = subarray.open_row((0, 13), 100, 10, 30, 20);
        assert_eq!((10, 200), result);
        assert_eq!(Some(5), subarray.opened_row);
        assert_eq!(12, subarray.last_read_col);
    }

    #[test]
    fn test_vec_add() {
        init_logger_debug();
        let cs_vec1 = CsVec::new(100, vec![1, 2, 3], vec![Pattern; 3]);
        let cs_vec2 = CsVec::new(100, vec![1, 3, 4], vec![Pattern; 3]);
        let result = sparse_add(cs_vec1.view(), cs_vec2.view());
        debug!(?result);
    }

    #[test]
    fn test_compute_bank_cycle_seq() {
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
                num: 4,
                ..Default::default()
            },
            graph_path: vec!["mtx/test.mtx".to_string()],
            ..Config::from_ddr4(
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
        let single_matrix: CsMat<Pattern> = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();
        let input_matrix = single_matrix.transpose_view().to_csr();
        let result = compute_bank_cycle_seq::<ddr4::Level>(
            &config,
            "single.mtx".to_string(),
            &single_matrix,
            &input_matrix,
            0,
            &ddr4::Storage::new(1, 1, 1, 1, 1, 100, 200, 200),
        );
        debug!(?result);
    }
}
