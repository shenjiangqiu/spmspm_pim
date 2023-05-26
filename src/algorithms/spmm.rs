//! ## rust module
//! ## Author: Jiangqiu Shen
//! ## Date: 2023-05-18
//! Description: the spmm algorithm
use sprs::{num_kinds::Pattern, CsMatViewI};

use super::{FrontierType, SpmvAlgorithm};
pub struct Spmm<'a> {
    current_frontier: usize,
    matrix: CsMatViewI<'a, Pattern, u32>,
}

impl<'a> Spmm<'a> {
    /// ## rust function
    /// ## Author: Jiangqiu Shen
    /// ## Date: 2023-05-18
    /// Description: create a new bfs algorithm with some matrix, and the init frontier is the first row of the matrix
    pub fn new(matrix: CsMatViewI<'a, Pattern, u32>) -> Self {
        Self {
            current_frontier: 0,
            matrix,
        }
    }
}

impl<'a> SpmvAlgorithm for Spmm<'a> {
    fn next_frontier(&mut self) -> Option<FrontierType> {
        if let Some(frontier) = self.matrix.outer_view(self.current_frontier) {
            self.current_frontier += 1;
            Some(FrontierType::Some(frontier.to_owned()))
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use sprs::{CsMatI, CsVecI};

    use super::*;

    #[test]
    fn test_next_frontier() {
        let matrix = CsMatI::new_csc((3, 3), vec![0, 1, 2, 3], vec![0, 1, 2], vec![Pattern; 3]);
        let mut spmm = Spmm::new(matrix.view());
        assert_eq!(
            spmm.next_frontier(),
            Some(FrontierType::Some(CsVecI::new(3, vec![0], vec![Pattern])))
        );
        assert_eq!(
            spmm.next_frontier(),
            Some(FrontierType::Some(CsVecI::new(3, vec![1], vec![Pattern])))
        );
        assert_eq!(
            spmm.next_frontier(),
            Some(FrontierType::Some(CsVecI::new(3, vec![2], vec![Pattern])))
        );
        assert_eq!(spmm.next_frontier(), None);
    }
}
