//! Mapping for same subarray
//!
#![allow(dead_code, unused)]
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
    fn get_matrix_b_location(
        &self,
        mat_b_row_id: LogicRowId,
    ) -> (SubarrayId, PhysicRowId, PhysicColId) {
        todo!()
    }

    fn get_result_dense_location(
        &self,
        target_row_id: LogicRowId,
        col_id: LogicColId,
    ) -> (SubarrayId, PhysicRowId, PhysicColId) {
        todo!()
    }

    fn get_matrix_b_location_with_shift(
        &self,
        mat_b_row_id: LogicRowId,
        shift: usize,
    ) -> (SubarrayId, PhysicRowId, PhysicColId) {
        todo!()
    }
}
