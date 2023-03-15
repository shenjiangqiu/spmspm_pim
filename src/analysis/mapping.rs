//! the module defines the trait Mapping
//!
//! A mapping is a function from a logic id to physical id
pub mod same_subarray;

macro_rules! generate_id_usize {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug)]
            pub struct $name(usize);
        )+
    };
}
generate_id_usize!(
    LogicRowId,
    LogicColId,
    PhysicRowId,
    SubarrayId,
    RingId,
    RingBufferId,
    TsvId,
);

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct RingPort(u8);

/// the mapping from a logic id to physical id
pub trait Mapping {
    /// if the row is evil row, get the physic row id
    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId;

    fn get_tsv_id_from_subarray(&self, sub_array_id: SubarrayId) -> TsvId;
    fn get_tsv_id_from_ring(&self, ring_id: RingId) -> TsvId;
    fn ring_port_from_subarray(&self, subarray_id: SubarrayId) -> RingPort;
    fn ring_buffer_id(&self, subarray_id: SubarrayId) -> RingBufferId;
    fn ring_id_from_subarray(&self, partition_id: SubarrayId) -> RingId;
    fn get_row_id(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId;
    fn get_row_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId;
    fn get_col_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> usize;
    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId;
    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId;
}
