//! the module defines the trait Mapping
//!
//! A mapping is a function from a logic id to physical id

use super::remap_analyze::row_cycle::*;
pub mod same_bank;
pub mod same_bank_weighted;
pub mod same_subarray;

pub trait Mapping {
    fn get_matrix_b_location(
        &self,
        mat_b_row_id: LogicRowId,
    ) -> (SubarrayId, PhysicRowId, PhysicColId);
    fn get_matrix_b_location_with_shift(
        &self,
        mat_b_row_id: LogicRowId,
        shift: usize,
    ) -> (SubarrayId, PhysicRowId, PhysicColId);
    /// the dense row id
    fn get_result_dense_location(
        &self,
        target_row_id: LogicRowId,
        col_id: LogicColId,
    ) -> (SubarrayId, PhysicRowId, PhysicColId);
}
