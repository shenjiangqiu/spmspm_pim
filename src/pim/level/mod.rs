//! the trait definition of a level spec
use std::fmt::Debug;

use sprs::{num_kinds::Pattern, CsMat};

pub mod ddr4;
pub mod hbm;

/// a trait that can represent a level in a dram
pub trait LevelTrait: Sized + Clone + Copy + Debug {
    /// the number of levels in a dram
    const LEVELS: usize;
    /// the storage type of the level spec
    type Storage: Debug + Clone;
    /// the mapping type that stores the mapping from a matrix a to dram location
    type Mapping;
    /// return is the level is bank
    fn is_bank(&self) -> bool;
    /// return is the level is channel
    fn is_channel(&self) -> bool;
    /// return is last
    fn is_last(&self) -> bool;
    /// return the next level
    fn get_child_level(&self) -> Option<Self>;
    ///
    fn to_usize(self) -> usize;
    ///
    fn first_level() -> Self;
    /// the last level to receive task
    fn last_level() -> Self;
    ///
    fn row() -> Self;

    /// get the specific id of a level
    fn get_level_id(&self, path: &Self::Storage) -> usize;
    /// get sub path to a level
    fn get_sub_path_to_level(&self, path: &Self::Storage) -> Self::Storage;
    /// get the total number of sub arrays when self is total size
    fn get_total_level(&self, total_size: &Self::Storage) -> usize;
    /// get the total number of sub arrays when self is total size
    fn get_flat_level_id(&self, total_size: &Self::Storage, id: &Self::Storage) -> usize;

    fn get_mapping(total_size: &Self::Storage, graph: &CsMat<Pattern>) -> Self::Mapping;
    fn get_row_detail(mapping: &Self::Mapping, row: usize) -> &GraphBRow<Self::Storage>;
}

/// the infomation of a row in matrix

#[derive(Debug)]
pub struct GraphBRow<Storage> {
    /// the path to the row in dram
    pub path: Storage,
    /// the size of the row(bytes)
    pub size: usize,
    /// the number of non-zero elements in the row
    pub nnz: usize,
}
