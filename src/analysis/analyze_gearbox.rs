//! this module is used to analyze the gearbox
//! # WARNING:
//!
//! !!! this module is derived from analyze_split_spmm.rs and the code and ***doc*** might not be accurate
use std::fmt::{Debug, Display};

use itertools::Itertools;

use serde::Serialize;
use sprs::{num_kinds::Pattern, CsMat};
use tracing::info;

use crate::{
    analysis::split::{split_matrix_by_col, split_matrix_by_row, NnzStats},
    pim::{
        config::Config,
        level::{ddr4, LevelTrait},
    },
};

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

fn analyze_gearbox_inner<LevelType: LevelTrait>(
    config: &Config,
    total_size: &LevelType::Storage,
) -> SplitAnalyzeResult
where
    LevelType::Storage: Debug,
    LevelType::Mapping: Debug,
{
    let total_graphs = config.graph_path.len();
    let results = config
        .graph_path
        .iter()
        .enumerate()
        .map(|(index, path)| {
            // will map the graph path to graph analysis result
            // start to analyze the graph
            info!("analyzing graph {}/{}", index + 1, total_graphs);
            // each subarray is a partition
            let partitions = config.channels.num
                * config.ranks.num
                * config.chips.num
                * config.bank_groups.num
                * config.banks.num
                * config.subarrays;
            // we should partition the matrix A in collumns
            let matrix_a: CsMat<Pattern> = sprs::io::read_matrix_market(path).unwrap().to_csc();
            let matrix_b = matrix_a.transpose_view().to_owned();
            assert!(matrix_b.storage() == sprs::CompressedStorage::CSR);
            let cols = matrix_a.cols();

            // evenly split the matrix
            let s_matrix_a = split_matrix_by_col(
                matrix_a,
                (0..partitions).map(|i| i * cols / partitions).collect_vec(),
            );
            // println!("split matrix {}", s_matrix);
            // average and mean man max nnz for sub matrix
            // println!("sub matrix nnz stats:{:?}", s_matrix.nnz_stats());
            let rows = matrix_b.rows();
            // evenly split the matrix b by rows
            let s_matrix_b = split_matrix_by_row(
                matrix_b,
                (0..partitions).map(|i| i * rows / partitions).collect_vec(),
            );
            assert_eq!(s_matrix_a.matrix.len(), s_matrix_b.matrix.len());
            info!(
                "start to compute the split spmm,num of partitions: {}",
                s_matrix_b.matrix.len()
            );
            let graph_result = s_matrix_a
                .matrix
                .iter()
                .zip(s_matrix_b.matrix.iter())
                .enumerate()
                .map(|(partition_id, (matrix_a, matrix_b))| {
                    info!("computing bank {partition_id}/{}", s_matrix_a.matrix.len());
                    compute_bank_cycle_seq::<LevelType>(
                        config,
                        path.to_string(),
                        matrix_b,
                        matrix_a,
                        total_size,
                    )
                })
                .collect_vec();

            SingleResult {
                name: path.to_string(),
                nnz_stats_a: s_matrix_a.nnz_stats(),
                nnz_stats_b: s_matrix_b.nnz_stats(),
                graph_result,
            }
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

/// compute the run cycles for each bank in sequence
///
/// in sequence means that tasks are executed in the order of the input,
/// the bank will read the tasks one by one and read the row of the output and input, and then
///  accumulate the input into the result.
///
/// # Arguments
/// - `config`: the config
/// - `matrix_b`: the matrix b belongs to the partition
/// - `matrix_a`: the input matrix the belons to the partition
/// - `bank_id`: the bank id
///
/// # Returns
/// - [`SeqResult`]: the stats
///  
pub fn compute_bank_cycle_seq<LevelType: LevelTrait>(
    config: &Config,
    path: String,
    matrix_b: &CsMat<Pattern>,
    matrix_a: &CsMat<Pattern>,
    total_size: &LevelType::Storage,
) -> SeqResult
where
    LevelType::Storage: Debug,
    LevelType::Mapping: Debug,
{
    // // initialize the statistics
    // let mut cycle: u64 = 0;
    // let mut compute_cycle: u64 = 0;
    // let mut temp_result_read: u64 = 0;
    // let mut final_result_write: u64 = 0;
    // let mut matrix_b_read: u64 = 0;

    // let mut row_open_bytes: usize = 0;
    // let mut used_bytes: usize = 0;

    // let mut input_read_bytes: usize = 0;
    // let mut input_read_times: usize = 0;

    // // first we need to map the matrix to the bank
    // // reset to 1 until subarray
    // debug!(?matrix_a);
    // debug!(?matrix_b);
    // // create the bank mapping for the matrix b
    // let mappings_b = LevelType::get_mapping(
    //     &LevelType::set_one_to_level(total_size, &LevelType::last_level()),
    //     matrix_b,
    // );
    // debug!(?mappings_b);

    // // assume we have the two sub arrays to store the partial result(it's not affecting the cycle accuracy)
    // let mut temp_result_subarray = SubarrayStatus::default();
    // let mut final_result_subarray = SubarrayStatus::default();
    // // a map from subarray to status
    // let mut open_row_status: hashbrown::HashMap<usize, SubarrayStatus> = Default::default();
    // for task in matrix_a.outer_iterator() {
    //     debug!("------start a task------");
    //     debug!(?task);
    //     // mean the result should be write to the temporary result subarray.
    //     let mut reverse_result = task.nnz() % 2 == 0;
    //     for (task_id_b, _) in task.iter() {
    //         debug!("------start a acc task------");
    //         debug!(?task_id_b);
    //         // first determine current round temp and final
    //         let (current_temp, current_final) = if reverse_result {
    //             (&mut final_result_subarray, &mut temp_result_subarray)
    //         } else {
    //             (&mut temp_result_subarray, &mut final_result_subarray)
    //         };
    //         reverse_result = !reverse_result;

    //         let input_row = matrix_b.outer_view(task_id_b).unwrap();
    //         debug!(?input_row);
    //         // the cycle to read temp result(open row)
    //         let (temp_result1, temp_result2, _opened_rows) = current_temp.open_row(
    //             (0, 0),
    //             current_result.nnz() * 4,
    //             config.activate_cycle as usize,
    //             config.precharge_cycle as usize,
    //             config.columns,
    //         );
    //         // row_open_bytes += opened_rows * config.columns;
    //         // used_bytes += current_result.nnz() * 4;
    //         temp_result_read += temp_result1 as u64 + temp_result2 as u64;
    //         debug!(?temp_result1, ?temp_result2);

    //         // the cycle to write final result(open row)
    //         let (final_result1, final_result2, _opened_rows) = current_final.open_row(
    //             (0, 0),
    //             current_result.nnz() * 4,
    //             config.activate_cycle as usize,
    //             config.precharge_cycle as usize,
    //             config.columns,
    //         );
    //         // row_open_bytes += opened_rows * config.columns;
    //         // used_bytes += current_result.nnz() * 4;
    //         final_result_write += final_result1 as u64 + final_result2 as u64;
    //         debug!(?final_result1, ?final_result2);
    //         // calculate the cycle:
    //         // 1. the cycle to calculate the nnz
    //         let _compute_cycle = current_result.nnz() as u64;
    //         cycle += _compute_cycle;
    //         compute_cycle += _compute_cycle;
    //         // 2. the cycle to open the row of the input matrix
    //         let input_row_detail = LevelType::get_row_detail(&mappings_b, task_id_b);
    //         let path = &input_row_detail.path;
    //         let subarray_id = LevelType::subarray().get_level_id(path);
    //         let row_id = LevelType::row().get_level_id(path);
    //         let col_id = LevelType::col().get_level_id(path);
    //         let input_subarray = open_row_status.entry(subarray_id).or_default();
    //         // the cycle to open the row of the input matrix
    //         let (input_cycle1, input_cycle2, opened_rows) = input_subarray.open_row(
    //             (row_id, col_id),
    //             input_row_detail.size,
    //             config.activate_cycle as usize,
    //             config.precharge_cycle as usize,
    //             config.columns,
    //         );
    //         row_open_bytes += opened_rows * config.columns;
    //         used_bytes += input_row_detail.size;
    //         input_read_bytes += input_row_detail.size;
    //         input_read_times += 1;
    //         matrix_b_read += input_cycle1 as u64 + input_cycle2 as u64;
    //         debug!(?input_cycle1, ?input_cycle2);
    //         // the first row open can be parallel, so we only count the max cycle
    //         cycle += temp_result1.max(final_result1).max(input_cycle1) as u64;
    //         // other row switch should be sequential
    //         cycle += (temp_result2 + final_result2 + input_cycle2) as u64;
    //     }
    // }

    // SeqResult {
    //     cycle,
    //     name: path,
    //     compute_cycle,
    //     temp_result_read,
    //     final_result_write,
    //     matrix_b_read,
    //     row_open: temp_result_read + final_result_write + matrix_b_read,
    //     row_open_bytes,
    //     used_bytes,
    //     input_read_bytes,
    //     input_read_times,
    // }
    todo!()
}

/// compute the run cycles for each bank in parallel
///
/// # Arguments
/// - `config`: the config
/// - `single_matrix`: the matrix b belongs to the bank
/// - `input_matrix`: the input matrix
/// - `bank_id`: the bank id
#[allow(dead_code)]
pub fn compute_bank_cycle_parallel<LevelType: LevelTrait>(
    _config: &Config,
    _single_matrix: CsMat<Pattern>,
    _input_matrix: &CsMat<Pattern>,
    _bank_id: &LevelType::Storage,
) -> u64 {
    todo!()
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
        let single_matrix: CsMat<Pattern> = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();
        let input_matrix = single_matrix.transpose_view().to_csr();
        let result = compute_bank_cycle_seq::<ddr4::Level>(
            &config,
            "single.mtx".to_string(),
            &single_matrix,
            &input_matrix,
            &ddr4::Storage::new(1, 1, 1, 1, 1, 100, 200, 200),
        );
        debug!(?result);
    }
}
