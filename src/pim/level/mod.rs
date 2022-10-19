//! the trait definition of a level spec
use std::fmt::Debug;

use sprs::{num_kinds::Pattern, CsMat};

pub mod ddr4;
pub mod hbm;

/// the storage type that represent a path to the last lelvel location of a dram type
pub trait PathStorage: Debug {
    /// the dram type
    type LevelType: LevelTrait;
    /// get the specific id of a level
    fn get_level_id(&self, level: &Self::LevelType) -> usize;
}

/// a trait that can represent a level in a dram
pub trait LevelTrait: Sized + Clone + Copy {
    /// the number of levels in a dram
    const LEVELS: usize;
    /// the storage type of the level spec
    type Storage: PathStorage<LevelType = Self>;
    /// the mapping type that stores the mapping from a matrix a to dram location
    type Mapping: MatrixBMapping<Storage = Self::Storage>;
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
pub trait MatrixBMapping {
    type Storage: PathStorage;
    fn get_mapping(total_size: &Self::Storage, graph: &CsMat<Pattern>) -> Self;
    fn get_row_detail(&self, row: usize) -> &GraphBRow<Self::Storage>;
}
