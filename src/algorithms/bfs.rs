//! ## rust module
//! ## Author: Jiangqiu Shen
//! ## Date: 2023-05-18
//! Description: the bfs algorithm
use bit_vec::BitVec;
use sprs::{num_kinds::Pattern, CsMatViewI, CsVecI};

use super::{FrontierType, SpmvAlgorithm};
enum RunningStatus {
    Running(Option<CsVecI<Pattern, u32>>),
    End,
}
pub struct Bfs<'a> {
    current_frontier: RunningStatus,
    matrix: CsMatViewI<'a, Pattern, u32>,
    visited_nodes: BitVec,
}

impl<'a> Bfs<'a> {
    /// ## rust function
    /// ## Author: Jiangqiu Shen
    /// ## Date: 2023-05-18
    /// Description: create a new bfs algorithm with some matrix, and the init frontier is the first row of the matrix
    pub fn new(matrix: CsMatViewI<'a, Pattern, u32>) -> Self {
        Self {
            current_frontier: RunningStatus::Running(None),
            matrix,
            visited_nodes: BitVec::from_elem(matrix.cols(), false),
        }
    }
}

impl<'a> SpmvAlgorithm for Bfs<'a> {
    fn next_frontier(&mut self) -> Option<FrontierType> {
        match self.current_frontier {
            RunningStatus::Running(ref mut current_frontier) => {
                match current_frontier.take() {
                    Some(frontier) => {
                        let mut next_frontier = vec![];
                        for (row_id, _) in frontier.iter() {
                            for &col_id in self.matrix.outer_view(row_id).unwrap().indices() {
                                if !self.visited_nodes.get(col_id as usize).unwrap() {
                                    next_frontier.push(col_id);
                                    self.visited_nodes.set(col_id as usize, true);
                                }
                            }
                        }
                        let len = next_frontier.len();
                        if len == 0 {
                            self.current_frontier = RunningStatus::End;
                            None
                        } else {
                            next_frontier.sort_unstable();
                            let next_frontier =
                                CsVecI::new(self.matrix.cols(), next_frontier, vec![Pattern; len]);
                            *current_frontier = Some(next_frontier.clone());
                            Some(FrontierType::Some(next_frontier))
                        }
                    }
                    None => {
                        // init frontier

                        let next_frontier = CsVecI::new(self.matrix.cols(), vec![0], vec![Pattern]);
                        self.visited_nodes.set(0, true);
                        *current_frontier = Some(next_frontier.clone());
                        Some(FrontierType::Some(next_frontier))
                    }
                }
            }
            RunningStatus::End => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use sprs::{num_kinds::Pattern, CsMatI};

    use crate::algorithms::SpmvAlgorithm;

    use super::Bfs;

    #[test]
    fn test_bfs() {
        let matrix: CsMatI<Pattern, u32> = sprs::io::read_matrix_market("test_mtx/test_large.mtx")
            .unwrap()
            .to_csr();
        let mut bfs = Bfs::new(matrix.view());
        println!("{:?}", matrix);
        while let Some(frontier) = bfs.next_frontier() {
            println!("{:?}", frontier);
        }
    }
}
