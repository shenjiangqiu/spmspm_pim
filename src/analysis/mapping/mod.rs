//! the module defines the trait Mapping
//!
//! A mapping is a function from a logic id to physical id
pub mod same_bank;
pub mod same_subarray;
macro_rules! generate_id_usize {
    ($($name:ident),+ $(,)?) => {
        $(
            /// a wrapper for the id
            #[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug)]
            #[repr(transparent)]
            pub struct $name(pub usize);
            impl std::ops::Deref for $name {
                type Target = usize;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
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
pub struct RingPort(pub u8);

/// the mapping from a logic id to physical id
pub trait Mapping {
    /// if the row is evil row, get the physic row id
    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId;
    /// get the tsv id from the global subarray id
    fn get_tsv_id_from_subarray(&self, sub_array_id: SubarrayId) -> TsvId;
    /// get the tsv id from the ring id
    fn get_tsv_id_from_ring(&self, ring_id: RingId) -> TsvId;
    fn ring_port_from_subarray(&self, subarray_id: SubarrayId) -> RingPort;
    /// get the ring buffer id from the global subarray id
    fn ring_buffer_id(&self, subarray_id: SubarrayId) -> RingBufferId;
    /// the ring id from the global subarray id
    fn ring_id_from_subarray(&self, partition_id: SubarrayId) -> RingId;
    /// the sparse row id
    fn get_row_id(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId;
    /// the dense row id
    fn get_row_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId;
    /// this is the dense col id
    fn get_col_id_dense(&self, target_row_id: LogicRowId, col_id: LogicColId) -> usize;
    /// will return the global subarray id
    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId;
    /// will return the global subarray id
    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId;
}
