use bit_vec::BitVec;
use sprs::{num_kinds::Pattern, CsMatViewI, CsVecI};

use super::SpmvAlgorithm;
enum RunningStatus {
    Running(Option<CsVecI<Pattern, u32>>),
    End,
}
pub struct Spknn<'a> {
    current_frontier: RunningStatus,
    matrix: CsMatViewI<'a, Pattern, u32>,
    visited_nodes: BitVec,
}
impl<'a> Spknn<'a> {
    pub fn new(matrix: CsMatViewI<'a, Pattern, u32>) -> Self {
        Self {
            current_frontier: RunningStatus::Running(None),
            matrix,
            visited_nodes: BitVec::from_elem(matrix.cols(), false),
        }
    }
}
impl<'a> SpmvAlgorithm for Spknn<'a> {
    fn next_frontier(&mut self) -> Option<CsVecI<Pattern, u32>> {
        todo!()
    }
}
