//! this module is used to analyze the split spmm

use rayon::prelude::*;
use std::{
    fmt::{Debug, Display},
    ops::Add,
};

use itertools::Itertools;

use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMat, CsVec, CsVecView};
use tracing::{debug, info};

use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
};

use super::split::{split_matrix_by_col, NnzStats};

/// the statistics of a single graph
#[derive(Serialize, Deserialize)]
pub struct SingleResult {
    /// the name of the graph
    pub name: String,
    /// the nnz statistics of the graph
    pub nnz_stats: NnzStats,
    /// the cycle and other stats for a graph
    pub graph_result: Vec<SeqResult>,
}
#[derive(Serialize, Deserialize)]
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
            println!("nnz_stats: {:?}", result.nnz_stats);
            for SeqResult {
                cycle,
                meta_cycle,
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
                row_open_no_overlap,
                ignore_empty_row_meta_cycle,
                total_cycle_ignore_empty_meta,
                total_cycle_fix_empty_meta,
                fix_empty_meta_cycle,
                total_non_empt_row,
                total_empty_row,
                total_cycle_ignore_meta,
            } in &result.graph_result
            {
                println!("cycle: {}", cycle);
                println!("meta_cycle: {}", meta_cycle);
                println!(
                    "ignore_empty_row_meta_cycle: {}",
                    ignore_empty_row_meta_cycle
                );
                println!("total_cycle_ignore_meta: {}", total_cycle_ignore_meta);
                println!(
                    "total_cycle_ignore_empty_meta: {}",
                    total_cycle_ignore_empty_meta
                );
                println!("total_cycle_fix_empty_meta: {}", total_cycle_fix_empty_meta);
                println!("fix_empty_meta_cycle: {}", fix_empty_meta_cycle);

                println!("comp_cycle: {}", compute_cycle);
                println!("row_open: {}", row_open);
                println!("row_open_no_overlap: {}", row_open_no_overlap);
                println!("temp_result_read: {}", temp_result_read);
                println!("final_result_write: {}", final_result_write);
                println!("matrix_b_read: {}", matrix_b_read);
                println!("row_open_bytes: {}", row_open_bytes);
                println!("used_bytes: {}\n", used_bytes);
                println!("input_read_bytes: {}", input_read_bytes);
                println!("input_read_times: {}\n", input_read_times);
                println!("total_non_empt_row: {}", total_non_empt_row);
                println!("total_empty_row: {}", total_empty_row);
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
            writeln!(f, "nnz_stats: {:?}", result.nnz_stats)?;
        }
        Ok(())
    }
}

/// analyze the split spmm
pub(crate) fn analyze_split_spmm(config: &Config) -> SplitAnalyzeResult {
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

fn analyze_split_spmm_inner<LevelType: LevelTrait>(
    config: &Config,
    total_size: &LevelType::Storage,
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
            let partitions = config.channels.num
                * config.ranks.num
                * config.chips.num
                * config.bank_groups.num
                * config.banks.num;
            let matrix_a: CsMat<Pattern> = sprs::io::read_matrix_market(path).unwrap().to_csr();
            let totoal_nnz = matrix_a.nnz();
            let matrix_b = matrix_a.transpose_view().to_owned();
            assert_eq!(matrix_b.storage(), sprs::CompressedStorage::CSC);
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
            let s_matrix = split_matrix_by_col(matrix_b, real_split_points);
            // println!("split matrix {}", s_matrix);
            // average and mean man max nnz for sub matrix
            // println!("sub matrix nnz stats:{:?}", s_matrix.nnz_stats());
            info!(
                "start to compute the split spmm,num of banks: {}",
                s_matrix.matrix.len()
            );
            let graph_result = s_matrix
                .matrix
                .par_iter()
                .enumerate()
                .map(|(bank_id, single_matrix)| {
                    info!("computing bank {bank_id}/{}", s_matrix.matrix.len());
                    compute_bank_cycle_seq::<LevelType>(
                        config,
                        path.to_string(),
                        single_matrix,
                        &matrix_a,
                        total_size,
                    )
                })
                .collect();

            SingleResult {
                name: path.to_string(),
                nnz_stats: s_matrix.nnz_stats(),
                graph_result,
            }
        })
        .collect_vec();
    SplitAnalyzeResult { results }
}

/// the stat result of the seq spmm
#[derive(Debug, Serialize, Deserialize)]
pub struct SeqResult {
    /// the cycles
    pub cycle: u64,
    /// meta data cycles
    pub total_cycle_ignore_meta: u64,
    pub meta_cycle: u64,
    pub ignore_empty_row_meta_cycle: u64,
    pub total_cycle_ignore_empty_meta: u64,
    pub total_cycle_fix_empty_meta: u64,
    pub fix_empty_meta_cycle: u64,
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
    pub total_non_empt_row: u64,
    pub total_empty_row: u64,
}

/// add two vector and return a new vector(sparse)
/// # Example
/// ```
/// use spmspm_pim::analysis::analyze_split_spmm;
/// use sprs::{CsVec, CsVecView};
/// let v1 = CsVec::new(5, vec![0, 2, 4], vec![1,1,1]);
/// let v2 = CsVec::new(5, vec![1, 3, 4], vec![1,1,1]);
/// let v3 = analyze_split_spmm::sparse_add(v1.view(), v2.view());
/// assert_eq!(v3, CsVec::new(5, vec![0, 1, 2, 3, 4], vec![1,1,1,1,2]));
/// ```
pub fn sparse_add<T>(v1: CsVecView<T>, v2: CsVecView<T>) -> CsVec<T>
where
    T: Add<Output = T> + Copy,
{
    assert_eq!(v1.dim(), v2.dim());
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
        cas: usize,
        columns: usize,
    ) -> (usize, usize, usize) {
        if length == 0 {
            return (0, 0, 0);
        }
        let mut total_rows_activated = 0;
        let first_row_cycle = match self.opened_row {
            Some(row) => {
                if row == start.0 {
                    cas
                } else {
                    total_rows_activated += 1;
                    activate_cycle + precharge_cycle + cas
                }
            }
            None => {
                total_rows_activated += 1;
                activate_cycle + cas
            }
        };
        // all remaining rows should be precharged and activated
        let final_col = start.1 + length;
        let remaining_rows = (final_col - 1) / columns;
        let final_row = start.0 + remaining_rows;
        let remaining_cycle = remaining_rows * (activate_cycle + precharge_cycle + cas);
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
/// - `matrix_b`: the matrix b belongs to the bank
/// - `matrix_a`: the input matrix
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
    // initialize the statistics
    let mut cycle: u64 = 0;
    let mut meta_cycle: u64 = 0;
    let mut compute_cycle: u64 = 0;
    let mut temp_result_read: u64 = 0;
    let mut final_result_write: u64 = 0;
    let mut matrix_b_read: u64 = 0;
    let mut ignore_empty_row_meta_cycle: u64 = 0;

    let mut total_cycle_ignore_empty_meta: u64 = 0;
    let mut total_cycle_fix_empty_meta: u64 = 0;
    let mut total_cycle_ignore_meta: u64 = 0;

    let mut fix_empty_meta_cycle: u64 = 0;
    let mut total_empty_row: u64 = 0;
    let mut total_non_empt_row: u64 = 0;
    let mut row_open_bytes: usize = 0;
    let mut used_bytes: usize = 0;
    let mut input_read_bytes: usize = 0;
    let mut input_read_times: usize = 0;
    let mut row_open: u64 = 0;
    // first we need to map the matrix to the bank
    // reset to 1 until subarray
    debug!(?matrix_a);
    debug!(?matrix_b);
    // create the bank mapping for the matrix b
    let mappings_b = LevelType::get_mapping(
        &LevelType::set_one_to_level(total_size, &LevelType::last_level()),
        matrix_b,
    );
    debug!(?mappings_b);

    // assume we have the two sub arrays to store the partial result(it's not affecting the cycle accuracy)
    let mut temp_result_subarray = SubarrayStatus::default();
    let mut final_result_subarray = SubarrayStatus::default();
    // the location to store the meta data(the ind_ptr)
    let mut metadata_subarray = SubarrayStatus::default();
    // a map from subarray to status
    let mut open_row_status: hashbrown::HashMap<usize, SubarrayStatus> = Default::default();
    for task in matrix_a.outer_iterator() {
        debug!("------start a task------");
        debug!(?task);
        let mut current_result: CsVec<Pattern> = CsVec::new(matrix_b.inner_dims(), vec![], vec![]);
        // mean the result should be write to the temporary result subarray.
        let mut reverse_result = task.nnz() % 2 == 0;
        for (task_id_b, _) in task.iter() {
            // first need to read the metadata to detect the location to read
            // we can assume that the ptr is located at address 0..size*4
            let ind_ptr_address = task_id_b * 4;
            let col_per_row = config.columns;
            let row_id = ind_ptr_address / col_per_row;
            let col_id = ind_ptr_address % col_per_row;
            // the cycles to read the meta data
            let (first_row_cycle, remaining_row_cycle, _) = metadata_subarray.open_row(
                (row_id, col_id),
                8,
                config.activate_cycle as usize,
                config.precharge_cycle as usize,
                config.cas as usize,
                config.columns,
            );
            cycle += first_row_cycle as u64 + remaining_row_cycle as u64;
            meta_cycle += first_row_cycle as u64 + remaining_row_cycle as u64;

            debug!("------start a acc task------");
            debug!(?task_id_b);
            debug!(?current_result);
            // first determine current round temp and final
            let input_row = matrix_b.outer_view(task_id_b).unwrap();
            if input_row.nnz() == 0 {
                // no need to work
                total_cycle_fix_empty_meta += 2;
                fix_empty_meta_cycle += 2;
                total_empty_row += 1;
                continue;
            }
            total_non_empt_row += 1;

            total_cycle_fix_empty_meta += first_row_cycle as u64 + remaining_row_cycle as u64;
            fix_empty_meta_cycle += first_row_cycle as u64 + remaining_row_cycle as u64;

            total_cycle_ignore_empty_meta += first_row_cycle as u64 + remaining_row_cycle as u64;
            ignore_empty_row_meta_cycle += first_row_cycle as u64 + remaining_row_cycle as u64;

            let (current_temp, current_final) = if reverse_result {
                (&mut final_result_subarray, &mut temp_result_subarray)
            } else {
                (&mut temp_result_subarray, &mut final_result_subarray)
            };
            reverse_result = !reverse_result;

            debug!(?input_row);
            // the cycle to read temp result(open row)
            let (temp_result1, temp_result2, _opened_rows) = current_temp.open_row(
                (0, 0),
                current_result.nnz() * 4,
                config.activate_cycle as usize,
                config.precharge_cycle as usize,
                config.cas as usize,
                config.columns,
            );
            // row_open_bytes += opened_rows * config.columns;
            // used_bytes += current_result.nnz() * 4;
            temp_result_read += temp_result1 as u64 + temp_result2 as u64;
            debug!(?temp_result1, ?temp_result2);

            current_result = sparse_add(current_result.view(), input_row);
            debug!(?current_result);
            // the cycle to write final result(open row)
            let (final_result1, final_result2, _opened_rows) = current_final.open_row(
                (0, 0),
                current_result.nnz() * 4,
                config.activate_cycle as usize,
                config.precharge_cycle as usize,
                config.cas as usize,
                config.columns,
            );
            // row_open_bytes += opened_rows * config.columns;
            // used_bytes += current_result.nnz() * 4;
            final_result_write += final_result1 as u64 + final_result2 as u64;
            debug!(?final_result1, ?final_result2);
            // calculate the cycle:
            // 1. the cycle to calculate the nnz
            let _compute_cycle = current_result.nnz() as u64;
            let mut cycle_compute_and_row_open = 0;
            cycle_compute_and_row_open += _compute_cycle;

            compute_cycle += _compute_cycle;
            // 2. the cycle to open the row of the input matrix
            let input_row_detail = LevelType::get_row_detail(&mappings_b, task_id_b);
            let path = &input_row_detail.path;
            let subarray_id = LevelType::subarray().get_level_id(path);
            let row_id = LevelType::row().get_level_id(path);
            let col_id = LevelType::col().get_level_id(path);
            let input_subarray = open_row_status.entry(subarray_id).or_default();
            // the cycle to open the row of the input matrix
            let (input_cycle1, input_cycle2, opened_rows) = input_subarray.open_row(
                (row_id, col_id),
                input_row_detail.size,
                config.activate_cycle as usize,
                config.precharge_cycle as usize,
                config.cas as usize,
                config.columns,
            );
            row_open_bytes += opened_rows * config.columns;
            used_bytes += input_row_detail.size;
            input_read_bytes += input_row_detail.size;
            input_read_times += 1;
            matrix_b_read += input_cycle1 as u64 + input_cycle2 as u64;
            debug!(?input_cycle1, ?input_cycle2);
            // the first row open can be parallel, so we only count the max cycle
            let first = temp_result1.max(final_result1).max(input_cycle1) as u64;
            cycle_compute_and_row_open += first;
            row_open += first;
            // other row switch should be sequential
            let others = (temp_result2 + final_result2 + input_cycle2) as u64;
            cycle_compute_and_row_open += others;
            row_open += others;

            cycle += cycle_compute_and_row_open;
            total_cycle_fix_empty_meta += cycle_compute_and_row_open;
            total_cycle_ignore_empty_meta += cycle_compute_and_row_open;
            total_cycle_ignore_meta += cycle_compute_and_row_open;
        }
    }

    SeqResult {
        cycle,
        total_cycle_ignore_empty_meta,
        total_cycle_fix_empty_meta,
        total_cycle_ignore_meta,
        fix_empty_meta_cycle,
        meta_cycle,
        ignore_empty_row_meta_cycle,
        name: path,
        compute_cycle,
        temp_result_read,
        final_result_write,
        matrix_b_read,
        row_open,
        row_open_no_overlap: temp_result_read + final_result_write + matrix_b_read,
        row_open_bytes,
        used_bytes,
        input_read_bytes,
        input_read_times,
        total_non_empt_row,
        total_empty_row,
    }
}

/// compute the run cycles for each bank in parallel
///
/// # Arguments
/// - `config`: the config
/// - `single_matrix`: the matrix b belongs to the bank
/// - `input_matrix`: the input matrix
/// - `bank_id`: the bank id
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
        let result = analyze_split_spmm(&config);
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

        let result = analyze_split_spmm(&config);
        result.show_results();
    }

    #[test]
    fn test_open_row() {
        init_logger_debug();
        let mut subarray = SubarrayStatus::default();
        let result = subarray.open_row((0, 13), 100, 10, 30, 22, 20);
        assert_eq!((10, 200, 0), result);
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
