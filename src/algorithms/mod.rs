//! ## rust module
//! ## Author: Jiangqiu Shen
//! ## Date: 2023-05-18
//! Description: this module contains some SPMV algorithms
pub mod bfs;
pub mod page_rank;
pub mod spmm;
pub mod sssp;
use sprs::{num_kinds::Pattern, CsVecI};

#[derive(Clone, Debug)]
pub enum FrontierType {
    All,
    Some(CsVecI<Pattern, u32>),
}

/// ## rust function
/// ## Author: Jiangqiu Shen
/// ## Date: 2023-05-18
/// Description: the trait of a spmv algorithm which defines the current frontier and how to generate the next frontier
pub trait SpmvAlgorithm {
    /// ## rust function
    /// ## Author: Jiangqiu Shen
    /// ## Date: 2023-05-18
    /// Description: move to the next frontier
    fn next_frontier(&mut self) -> Option<FrontierType>;
}
