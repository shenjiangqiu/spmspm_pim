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

#[derive(Default)]
struct BankStatus {
    opened_row: Option<usize>,
    last_read_col: usize,
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
    let mut final_result_bank = BankStatus::default();
    let mut final_result_bank = BankStatus::default();
    let mut open_row_status: hashbrown::HashMap<usize, BankStatus> = Default::default();
    trace!(?mappings);
    for task in input_matrix.outer_iterator() {
        let mut current_result: CsVec<Pattern> =
            CsVec::new(single_matrix.inner_dims(), vec![], vec![]);
        for (input_row_id, _) in task.iter() {
            let input_row = single_matrix.outer_view(input_row_id).unwrap();
            current_result = sparse_add(current_result.view(), input_row);
            // calculate the cycle:
            // the cycle to calculate the nnz
            cycle += current_result.nnz() as u64;
            // the cycle to open the row of the input matrix
            let open_row_detail = LevelType::get_row_detail(mapping, input_row_id);
            let sub_array_id = LevelType::sub_array().get_level_id(&open_row_detail.path);
            let row_id = LevelType::row().get_level_id(&open_row_detail.path);
            match open_row_status.entry(row_id) {
                hashbrown::hash_map::Entry::Occupied(entry) => {
                    let status = entry.get_mut();
                    let open_status = &mut status.opened_row;
                    match open_status {
                        Some(opened_row) => {
                            if opened_row != &open_row_detail.row {
                                // the row is not opened, we need to open it
                                cycle += config.open_row_cycle;
                                *opened_row = open_row_detail.row;
                            }
                        }
                        None => {}
                    }
                }
                hashbrown::hash_map::Entry::Vacant(_) => todo!(),
            }
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

    use tracing::info;

    use crate::{
        init_logger_debug,
        pim::config::{Config, LevelConfig},
    };

    use super::analyze_split_spmm;

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
}
