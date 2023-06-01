//! Mapping for same bank
//!
//! todo: implement this

use sprs::{num_kinds::Pattern, CsMatI};
use tracing::debug;

use crate::tools;

use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BitsField {
    bits: usize,
    offset: usize,
}

impl BitsField {
    #[allow(dead_code)]
    fn get(&self, addr: usize) -> usize {
        let mask = (1 << self.bits) - 1;
        (addr >> self.offset) & mask
    }
}
#[derive(Debug)]
pub struct RowIdMappingEntry {
    pub bank_id: usize,
    pub subarray_id: usize,
    pub row_id: usize,
    pub col_id: usize,
    pub size: usize,
}
impl RowIdMappingEntry {
    pub fn new(
        bank_id: usize,
        subarray_id: usize,
        row_id: usize,
        col_id: usize,
        size: usize,
    ) -> Self {
        Self {
            bank_id,
            subarray_id,
            row_id,
            col_id,
            size,
        }
    }
}
#[derive(Debug)]
struct DenseIdMappingEntry {
    pub bank_id: usize,
    pub subarray_id: usize,
    pub shift: usize,
}
/// the row and dense col share the same mapping
/// it follows the principles:
/// 1. we first partition the graph into several partitions, each partition have a range of row
///    ids,
/// 2. we set the num of the partitions to the number of the banks, and make sure the weight of
///    each partition are similar
/// 3. after the mapping, we create the layout of the mapping insde the bank,
#[derive(Debug)]
pub struct SameBankWeightedMapping {
    /// the mappings for matrix b rows
    row_id_mappings: Vec<RowIdMappingEntry>,
    /// the dense size for each subarray.
    subarray_dense_size: Vec<usize>,
    /// the mappings for the dense result
    dense_id_mapping: Vec<DenseIdMappingEntry>,
    subarray_bits: usize,
    col_bits: usize,
}
fn get_global_subarray_id_from_local_subarray_id(
    local_subarray_id: usize,
    bank_id: usize,
    subarray_bits: usize,
) -> SubarrayId {
    SubarrayId(bank_id << subarray_bits | local_subarray_id)
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
        col_size: usize,
        graph: &CsMatI<Pattern, u32>,
    ) -> Self {
        // first, distribute the rows to banks,
        // first calculate the average weight
        let weight = graph.nnz();
        let global_total_banks = total_channels * total_banks;

        let average_nnz: f32 = weight as f32 / global_total_banks as f32;
        debug!("total_nnz: {}", weight);
        debug!("average_nnz: {}", average_nnz);

        // then map the rows into banks
        let mut bank_rows = vec![];
        let mut accumualted_nnz = 0;
        // setup the bank rows to contains the start row id of each bank
        //
        // will make sure each bank have similar weight
        let mut row_id = 0;
        let mut graph_iter = graph.outer_iterator();
        'outer: for bank_id in 0..global_total_banks {
            let target_nnz = (bank_id + 1) as f32 * average_nnz;
            debug!("target_nnz for bank: {} :{}", bank_id, target_nnz);
            if accumualted_nnz as f32 > target_nnz {
                debug!(
                    "accumulated_nnz: {} is larger than: {}, push {} to bank {}",
                    accumualted_nnz, target_nnz, row_id, bank_id
                );
                bank_rows.push(row_id);
                continue;
            }
            for row in graph_iter.by_ref() {
                row_id += 1;
                accumualted_nnz += row.nnz();
                if accumualted_nnz as f32 > target_nnz {
                    debug!(
                        "accumulated_nnz: {} is larger than: {}, push {} to bank {}",
                        accumualted_nnz, target_nnz, row_id, bank_id
                    );
                    bank_rows.push(row_id);
                    continue 'outer;
                }
            }
            break;
        }
        while bank_rows.len() < global_total_banks {
            bank_rows.push(graph.rows());
        }

        debug!("bank_rows: {}", bank_rows.len());
        // now all bank have the rows, distribute them to the subarrays
        let mut row_id_mappings = vec![];
        let mut dense_id_mapping = vec![];
        let mut start_row_id = 0;
        let mut total_subarray_dense_size = vec![];
        // in this loop, will setup the row_id_mappings, which contains the detailed mappings for
        // each row!
        for (bank_id, end_row_id) in bank_rows.into_iter().enumerate() {
            debug!(
                "bank_id: {}, start_row_id: {}, end_row_id: {}",
                bank_id, start_row_id, end_row_id
            );
            // this represent a bank, first create the subarray status for each subarray
            #[derive(Debug, Clone, Copy)]
            struct SubarrayStatus {
                row_id: usize,
                col_id: usize,
            }
            let mut subarray_status = vec![
                SubarrayStatus {
                    row_id: 0,
                    col_id: 0,
                };
                total_subarrays
            ];
            let mut subarray_size = vec![0; total_subarrays];
            // for each row, put it into the subarray
            for (subarray_id, row_id) in (start_row_id..end_row_id)
                .enumerate()
                .map(|(index, row_id)| (index % total_subarrays, row_id))
            {
                debug!(
                    "bank_id: {}, subarray_id: {}, row_id: {}",
                    bank_id, subarray_id, row_id
                );
                // put that row into that subarray
                let subarray = &mut subarray_status[subarray_id];
                let subarray_row_id = subarray.row_id;
                let subarray_col_id = subarray.col_id;
                let row_len = graph.outer_view(row_id).unwrap().nnz();
                assert_eq!(row_id_mappings.len(), row_id);
                let shift = subarray_size[subarray_id];
                dense_id_mapping.push(DenseIdMappingEntry {
                    bank_id,
                    shift,
                    subarray_id,
                });
                row_id_mappings.push(RowIdMappingEntry::new(
                    bank_id,
                    subarray_id,
                    subarray_row_id,
                    subarray_col_id,
                    row_len * 4,
                ));

                // update the subarray

                let next_col = subarray_col_id + row_len * 4;
                let next_row = subarray_row_id + next_col / col_size;
                let next_col = next_col % col_size;
                subarray.row_id = next_row;
                subarray.col_id = next_col;
                subarray_size[subarray_id] += 1;
            }

            total_subarray_dense_size.extend(subarray_size);
            start_row_id = end_row_id;
        }

        // now we have the row_id_mappings, we can create the mapping
        // debug!("the row_id_mappings is {:?}", row_id_mappings);
        assert_eq!(row_id_mappings.len(), graph.rows());
        assert_eq!(dense_id_mapping.len(), graph.rows());
        assert_eq!(graph.rows(), graph.cols());
        let subarray_bits = tools::math::count_to_log(total_subarrays);
        let col_bits = tools::math::count_to_log(col_size);
        Self {
            col_bits,
            row_id_mappings,
            subarray_bits,
            subarray_dense_size: total_subarray_dense_size,
            dense_id_mapping,
        }
    }
}
impl Mapping for SameBankWeightedMapping {
    fn get_matrix_b_location(
        &self,
        mat_b_row_id: LogicRowId,
    ) -> (SubarrayId, PhysicRowId, PhysicColId) {
        let location = self.row_id_mappings.get(mat_b_row_id.0).unwrap();
        let subarray_id = location.subarray_id;
        let bank_id = location.bank_id;
        let subarray_id =
            get_global_subarray_id_from_local_subarray_id(subarray_id, bank_id, self.subarray_bits);
        (subarray_id, location.row_id.into(), location.col_id.into())
    }

    fn get_result_dense_location(
        &self,
        target_row_id: LogicRowId,
        col_id: LogicColId,
    ) -> (SubarrayId, PhysicRowId, PhysicColId) {
        let dense_id_mapping = self.dense_id_mapping.get(col_id.0).unwrap();
        let subarray_id = dense_id_mapping.subarray_id;
        let bank_id = dense_id_mapping.bank_id;
        let subarray_id =
            get_global_subarray_id_from_local_subarray_id(subarray_id, bank_id, self.subarray_bits);
        let shift = dense_id_mapping.shift;
        let size = self.subarray_dense_size[subarray_id.0];
        let real_shift = size * target_row_id.0 + shift;
        let real_shift = real_shift * 4;
        let row_id = real_shift >> self.col_bits;
        let col_id = real_shift & ((1 << self.col_bits) - 1);

        (subarray_id, row_id.into(), col_id.into())
    }

    fn get_matrix_b_location_with_shift(
        &self,
        mat_b_row_id: LogicRowId,
        shift: usize,
    ) -> (SubarrayId, PhysicRowId, PhysicColId) {
        let location = self.row_id_mappings.get(mat_b_row_id.0).unwrap();
        let col_id = location.col_id + shift * 4;
        let row_id = location.row_id + (col_id >> self.col_bits);
        let col_id = col_id & ((1 << self.col_bits) - 1);
        let subarray_id = location.subarray_id;
        let bank_id = location.bank_id;
        let subarray_id =
            get_global_subarray_id_from_local_subarray_id(subarray_id, bank_id, self.subarray_bits);
        (subarray_id, row_id.into(), col_id.into())
    }
}
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sprs::{num_kinds::Pattern, TriMatI};
    use tracing::metadata::LevelFilter;

    use crate::init_logger_stderr;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test() {
        init_logger_stderr(LevelFilter::DEBUG);
        let matrix: TriMatI<Pattern, u32> =
            sprs::io::read_matrix_market("test_mtx/bcspwr03.mtx").unwrap();
        let matrix = matrix.to_csr();
        let mapping = super::SameBankWeightedMapping::new(4, 4, 4, 4, &matrix);
        println!("{:?}", mapping);
    }
    /// .
    ///
    /// # Panics
    ///
    /// Panics if now graph is found.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_dist() {
        init_logger_stderr(LevelFilter::DEBUG);
        let matrix: TriMatI<Pattern, u32> =
            sprs::io::read_matrix_market("test_mtx/bcspwr03.mtx").unwrap();
        let matrix = matrix.to_csr();
        let mapping = super::SameBankWeightedMapping::new(4, 4, 4, 4, &matrix);
        let mut bank_counts = BTreeMap::new();
        let mut subarray_counts = BTreeMap::new();
        let mut subarray_counts_weight = BTreeMap::new();
        let mut bank_counts_weight = BTreeMap::new();
        for row_mapping in mapping.row_id_mappings.iter() {
            let bank_id = row_mapping.bank_id;
            let subarray_id = row_mapping.subarray_id;
            let subarray_id = bank_id << mapping.subarray_bits | subarray_id;
            let weight = row_mapping.size;
            *bank_counts.entry(bank_id).or_insert(0) += 1;
            *subarray_counts.entry(subarray_id).or_insert(0) += 1;
            *bank_counts_weight.entry(bank_id).or_insert(0) += weight;
            *subarray_counts_weight.entry(subarray_id).or_insert(0) += weight;
        }
        println!("bank_counts: {:?}", bank_counts);
        println!("subarray_counts: {:?}", subarray_counts);
        println!("bank_counts_weight: {:?}", bank_counts_weight);
        println!("subarray_counts_weight: {:?}", subarray_counts_weight);
    }
}
