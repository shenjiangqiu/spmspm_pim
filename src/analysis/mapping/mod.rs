//! the module defines the trait Mapping
//!
//! A mapping is a function from a logic id to physical id
pub mod same_bank;
pub mod same_bank_weighted;
pub mod same_subarray;

macro_rules! generate_id{
    ($t:ty;$($name:ident),+ $(,)?) => {
        $(
            /// a wrapper for the id
            #[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, )]
            #[repr(transparent)]
            pub struct $name(pub $t);
            impl std::fmt::Debug for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }
            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }
            impl std::ops::Deref for $name {
                type Target = $t;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
            impl std::ops::DerefMut for $name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
            impl std::convert::From<$t> for $name {
                fn from(id: $t) -> Self {
                    Self(id)
                }
            }
            impl std::convert::From<$name> for $t {
                fn from(id: $name) -> Self {
                    id.0
                }
            }

        )+
    };
}
generate_id!(
    usize;
    LogicRowId,
    LogicColId,
    PhysicRowId,
    PhysicColId,
    SubarrayId,
    RingId,
    RingBufferId,
    TsvId,
);
generate_id!(u8;RingPort);

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
