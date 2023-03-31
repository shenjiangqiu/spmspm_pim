//! the weited mapping make it more banlanced
//!
//!
//!
//! ```text
//!  __________________
//! < wish that works! >
//! ------------------
//!        \   ^__^
//!         \  (oo)\_______
//!            (__)\       )\/\
//!                ||----w |
//!                ||     ||
//! ```
use super::{
    LogicColId, LogicRowId, Mapping, PhysicRowId, RingBufferId, RingId, RingPort, SubarrayId, TsvId,
};

/// the weited mapping make it more banlanced
///
///
///
/// ```text
///  __________________
/// < wish that works/ >
/// ------------------
///        \   ^__^
///         \  (oo)\_______
///            (__)\       )\/\
///                ||----w |
///                ||     ||
/// ```
pub struct SameBankWeightedMapping {}

impl Mapping for SameBankWeightedMapping {
    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }

    fn get_tsv_id_from_subarray(&self, sub_array_id: SubarrayId) -> TsvId {
        //     TsvId(sub_array_id.0 / self.config.subarrays / self.config.banks.num)
        // should be the channel id
        // TsvId(sub_array_id.0 >> self.subarray_bits.bits >> self.bank_bits.bits)
        
        todo!()
    }

    fn get_tsv_id_from_ring(&self, ring_id: RingId) -> TsvId {
        //     TsvId(ring_id.0)
        // ring id is also the channel id
        TsvId(ring_id.0)
    }

    fn ring_port_from_subarray(&self, subarray_id: SubarrayId) -> RingPort {
        //     RingPort(((subarray_id.0 / self.config.subarrays) % self.config.banks.num) as u8)
        // ring port is the relative bank id
        // let id = (subarray_id.0 >> self.subarray_bits.bits) & ((1 << self.bank_bits.bits) - 1);
        todo!()
    }

    fn ring_buffer_id(&self, subarray_id: SubarrayId) -> RingBufferId {
        //     RingBufferId(subarray_id.0 / self.config.subarrays)
        todo!()
    }

    fn ring_id_from_subarray(&self, partition_id: SubarrayId) -> RingId {
        // ring id is the channel id

        // RingId(partition_id.0 >> self.subarray_bits.bits >> self.bank_bits.bits)
        todo!()
    }

    fn get_row_id(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        // PhysicRowId(self.get_row_rowid(mat_b_row_id.0))
        todo!()
    }

    fn get_row_id_dense(&self, _target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        //     let real_col_id =
        //         target_row_id.0 * self.col_per_partition * 4 + col_id.0 % self.col_per_partition;
        //     PhysicRowId(real_col_id / 256)
        // frist calculate howmany bits are used for the entire row of dense addr

        // then shift the row id to the left and plus the row addr.
        // ring buffer id is the absolute bank id
        // step 1, calculate the row size

        // PhysicRowId(self.get_col_rowid(col_id.0))
        todo!()
    }

    fn get_col_id_dense(&self, _target_row_id: LogicRowId, col_id: LogicColId) -> usize {
        // self.get_col_colid(col_id.0)
        todo!()
    }

    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId {
        // SubarrayId(self.get_global_subarray_id(row_id.0))
        todo!()
    }

    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId {
        // SubarrayId(self.get_global_subarray_id(col_id.0))
        todo!()
    }
}
