//! Mapping for same subarray
//!
//! todo: implement this

use super::Mapping;

struct SameSubarrayMapping {}
impl SameSubarrayMapping {}

impl Mapping for SameSubarrayMapping {
    fn get_row_id_evil(
        &self,
        mat_b_row_id: super::LogicRowId,
        _col_id: super::LogicColId,
    ) -> super::PhysicRowId {
        todo!()
    }

    fn get_tsv_id_from_subarray(&self, sub_array_id: super::SubarrayId) -> super::TsvId {
        todo!()
    }

    fn get_tsv_id_from_ring(&self, ring_id: super::RingId) -> super::TsvId {
        todo!()
    }

    fn ring_port_from_subarray(&self, subarray_id: super::SubarrayId) -> super::RingPort {
        todo!()
    }

    fn ring_buffer_id(&self, subarray_id: super::SubarrayId) -> super::RingBufferId {
        todo!()
    }

    fn ring_id_from_subarray(&self, partition_id: super::SubarrayId) -> super::RingId {
        todo!()
    }

    fn get_row_id(
        &self,
        mat_b_row_id: super::LogicRowId,
        _col_id: super::LogicColId,
    ) -> super::PhysicRowId {
        todo!()
    }

    fn get_row_id_dense(
        &self,
        target_row_id: super::LogicRowId,
        col_id: super::LogicColId,
    ) -> super::PhysicRowId {
        todo!()
    }

    fn get_col_id_dense(
        &self,
        target_row_id: super::LogicRowId,
        col_id: super::LogicColId,
    ) -> usize {
        todo!()
    }

    fn get_partition_id_row(&self, row_id: super::LogicRowId) -> super::SubarrayId {
        todo!()
    }

    fn get_partition_id_col(&self, col_id: super::LogicColId) -> super::SubarrayId {
        todo!()
    }
}
