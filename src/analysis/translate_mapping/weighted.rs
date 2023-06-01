//! weighted mapping to banlance the workload of each subarray
//!
//! finished

use crate::analysis::remap_analyze::row_cycle::*;
use std::cmp::Reverse;

use itertools::Itertools;
use sprs::{num_kinds::Pattern, CsMatI, TriMatI};
use tracing::debug;

use super::TranslateMapping;

/// the row and dense col share the same mapping
/// it follows the principles:
/// 1. we first partition the graph into several partitions, each partition have a range of row
///    ids,
/// 2. we set the num of the partitions to the number of the banks, and make sure the weight of
///    each partition are similar
/// 3. after the mapping, we create the layout of the mapping insde the bank,
#[derive(Debug)]
pub struct SameBankWeightedMapping {
    row_sub_mapping: super::AverageMapping,
    evil_threshold: usize,
}

impl TranslateMapping for SameBankWeightedMapping {
    type RowSubMapping = super::AverageMapping;

    fn get_row_sub_mapping(&self) -> &Self::RowSubMapping {
        &self.row_sub_mapping
    }

    fn is_evil(&self, row_id: LogicRowId) -> bool {
        row_id.0 < self.evil_threshold
    }
}

impl SameBankWeightedMapping {
    /// create the new mapping,
    /// in this mapping, it achieve the goles defined here [`SameBankWeightedMapping`]
    /// # Arguments
    /// * `total_rows` - the total number of rows
    /// * `total_banks` - the total number of banks in a channel
    /// * `total_channels` - the total number of channels
    /// * `total_subarrays` - the total number of subarrays in a bank
    /// * `col_size` - the size of the col in a subarray row
    /// * `graph` - the graph
    /// # Returns
    /// the mapping
    pub fn new(
        total_banks: usize,
        total_channels: usize,
        total_subarrays: usize,
        evil_threshold: usize,
        cols: usize,
        graph: &TriMatI<Pattern, u32>,
        graph_csr: &CsMatI<Pattern, u32>,
    ) -> (Self, CsMatI<Pattern, u32>) {
        let mut row_id_nnz = graph_csr
            .outer_iterator()
            .enumerate()
            .map(|(i, row)| (i, row.nnz()))
            .collect_vec();
        row_id_nnz.sort_unstable_by_key(|(_, nnz)| Reverse(*nnz));
        // distribute the non-evil to the banks
        let mut non_evil_mapping = vec![];
        let total_subarrays = total_channels * total_banks * total_subarrays;
        let (evil_rows, non_evil_rows) = row_id_nnz.split_at_mut(evil_threshold);
        non_evil_rows.sort_unstable_by_key(|(_i, nnz)| Reverse(*nnz));
        for i in 0..total_subarrays {
            let mut start_i = i;
            let mut start_j = 2 * total_subarrays - i - 1;
            loop {
                if start_i < non_evil_rows.len() {
                    debug!(
                        "put {}-{} to subarray {} ,nnz: {}",
                        start_i, non_evil_rows[start_i].0, i, non_evil_rows[start_i].1
                    );
                    non_evil_mapping.push(non_evil_rows[start_i].0);
                    start_i += total_subarrays * 2;
                } else {
                    break;
                }
                if start_j < non_evil_rows.len() {
                    debug!(
                        "put {}-{} to subarray {} ,nnz: {}",
                        start_j, non_evil_rows[start_j].0, i, non_evil_rows[start_j].1
                    );
                    non_evil_mapping.push(non_evil_rows[start_j].0);
                    start_j += total_subarrays * 2;
                } else {
                    break;
                }
            }
        }
        let mapping = evil_rows
            .iter()
            .map(|(i, _)| *i)
            .chain(non_evil_mapping)
            .collect_vec();
        let matrix_translated =
            crate::tools::remapping_translate::translate(graph.view(), &mapping).to_csr();

        let row_sub_mapping = super::AverageMapping::new(
            matrix_translated.view(),
            evil_threshold,
            total_subarrays,
            cols,
        );
        (
            Self {
                row_sub_mapping,
                evil_threshold,
            },
            matrix_translated,
        )
    }
}
#[cfg(test)]
mod tests {
    use sprs::{num_kinds::Pattern, TriMatI};
    use tracing::metadata::LevelFilter;

    use crate::{analysis::translate_mapping::TranslateMapping, init_logger_stderr};
    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_dist() {
        init_logger_stderr(LevelFilter::DEBUG);
        let matrix: TriMatI<Pattern, u32> =
            sprs::io::read_matrix_market("test_mtx/bcspwr03.mtx").unwrap();
        let (mapping, translated_matrix) =
            super::SameBankWeightedMapping::new(2, 2, 2, 4, 16, &matrix, &matrix.to_csr());
        let translated_csr = translated_matrix.to_csr();

        for (row_id, row) in translated_csr.outer_iterator().enumerate() {
            println!("row_id: {}, nnz: {}", row_id, row.nnz());
        }
        for i in 4..translated_csr.rows() {
            let location = mapping.get_location(i.into(), translated_csr.view());
            println!(
                "id: {}, nnz: {}",
                i,
                translated_csr.outer_view(i).unwrap().nnz()
            );
            println!("location: {:?}", location);
        }
        // test the evil location
        for i in 0..4 {
            let location = mapping.get_location_evil(i.into(), translated_csr.view());
            print!(
                "evil: id: {}, nnz: {} ",
                i,
                translated_csr.outer_view(i).unwrap().nnz()
            );
            println!("location: {:?}", location);
        }
        // test dense location
        for i in 0..translated_csr.rows() {
            let location = mapping.get_dense_location(0.into(), i.into(), translated_csr.view());
            print!("dense: id: {},  ", i,);
            println!("location: {:?}", location);
        }
    }
}
