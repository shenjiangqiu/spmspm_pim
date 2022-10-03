use std::fmt::Debug;

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
    fn is_bank(&self) -> bool;
    fn is_channel(&self) -> bool;
    fn is_last(&self) -> bool;
    fn get_child_level(&self) -> Option<Self>;
    fn to_usize(self) -> usize;
    fn first_level() -> Self;
    fn last_level() -> Self;
    fn row() -> Self;
}
