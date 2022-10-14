use std::fmt::Debug;

use sprs::{num_kinds::Pattern, CsMat};

pub mod ddr4;
pub mod hbm;

pub trait PathStorage: Debug {
    type LevelType: LevelTrait;
    fn get_level_id(&self, level: &Self::LevelType) -> usize;
}

/// a trait that can represent a level in a dram
pub trait LevelTrait: Sized + Clone + Copy {
    const LEVELS: usize;
    type Storage: PathStorage<LevelType = Self>;
    type Mapping: MatrixBMapping<Storage = Self::Storage>;
    fn is_bank(&self) -> bool;
    fn is_channel(&self) -> bool;
    fn is_last(&self) -> bool;
    fn get_child_level(&self) -> Option<Self>;
    fn to_usize(self) -> usize;
    fn first_level() -> Self;
    fn last_level() -> Self;
    fn row() -> Self;
}
pub struct GraphBRow<Storage> {
    pub path: Storage,
    pub size: usize,
}
pub trait MatrixBMapping {
    type Storage: PathStorage;
    fn get_mapping(total_size: &Self::Storage, graph: &CsMat<Pattern>) -> Self;
    fn get_row_detail(&self, row: usize) -> &GraphBRow<Self::Storage>;
}
