//! Mapping for same subarray
//!

use crate::pim::configv2::ConfigV2;

use super::*;

pub struct SameSubarrayMapping<'a> {
    config: &'a ConfigV2,
    row_per_partition: usize,
    col_per_partition: usize,
}

impl<'a> SameSubarrayMapping<'a> {
    pub fn new(config: &'a ConfigV2, row_per_partition: usize, col_per_partition: usize) -> Self {
        Self {
            config,
            row_per_partition,
            col_per_partition,
        }
    }
}

impl<'a> Mapping for SameSubarrayMapping<'a> {
    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }

    fn get_tsv_id_from_subarray(&self, sub_array_id: SubarrayId) -> TsvId {
        TsvId(sub_array_id.0 / self.config.subarrays / self.config.banks.num)
    }
    #[allow(unused)]
    fn get_tsv_id_from_ring(&self, ring_id: RingId) -> TsvId {
        // the ring id is the same as the tsv id
        TsvId(ring_id.0)
    }

    #[allow(dead_code)]
    fn ring_port_from_subarray(&self, subarray_id: SubarrayId) -> RingPort {
        RingPort(((subarray_id.0 / self.config.subarrays) % self.config.banks.num) as u8)
    }

    /// get the ring_buffer_id(bank id) from subarray id
    fn ring_buffer_id(&self, subarray_id: SubarrayId) -> RingBufferId {
        // return the global bank id
        RingBufferId(subarray_id.0 / self.config.subarrays)
    }
    /// from bank id to ring id
    fn ring_id_from_subarray(&self, partition_id: SubarrayId) -> RingId {
        let bank_id = partition_id.0 / self.config.subarrays;
        RingId(bank_id / self.config.banks.num)
    }

    fn get_row_id(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }

    /// fix a bug here, the one subarray do not contains the whole dense vec, so the col id should % self.col_per_partition
    fn get_row_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        let real_col_id =
            target_row_id.0 * self.col_per_partition * 4 + col_id.0 % self.col_per_partition;
        PhysicRowId(real_col_id / 256)
    }
    /// fix a bug here, the one subarray do not contains the whole dense vec, so the col id should % self.col_per_partition
    fn get_col_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> usize {
        let real_col_id =
            target_row_id.0 * self.col_per_partition * 4 + col_id.0 % self.col_per_partition;
        real_col_id % 256
    }

    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId {
        // the rows are distrubuted to every subarray
        SubarrayId(row_id.0 / self.row_per_partition)
    }

    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId {
        // the cols are distrubuted to every subarray
        SubarrayId(col_id.0 / self.col_per_partition)
    }
}
