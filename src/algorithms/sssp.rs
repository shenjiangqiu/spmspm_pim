use sprs::{num_kinds::Pattern, CsMatViewI, CsVecI};

use super::{FrontierType, SpmvAlgorithm};
enum RunningStatus {
    Running(Option<CsVecI<Pattern, u32>>),
    End,
}
pub struct Sssp<'a> {
    current_frontier: RunningStatus,
    matrix: CsMatViewI<'a, Pattern, u32>,
    current_length: Vec<u32>,
}

impl<'a> Sssp<'a> {
    pub fn new(matrix: CsMatViewI<'a, Pattern, u32>) -> Self {
        let mut current_length = vec![u32::MAX; matrix.cols()];
        current_length[0] = 0;
        Self {
            current_frontier: RunningStatus::Running(None),
            matrix,
            current_length,
        }
    }
}

impl<'a> SpmvAlgorithm for Sssp<'a> {
    fn next_frontier(&mut self) -> Option<FrontierType> {
        match self.current_frontier {
            RunningStatus::Running(ref mut frontier) => match frontier.take() {
                Some(f) => {
                    let mut next_frontier = CsVecI::new(self.matrix.cols(), vec![], vec![]);
                    for (row_id, _) in f.iter() {
                        for &col_id in self.matrix.outer_view(row_id).unwrap().indices() {
                            if self.current_length[col_id as usize] == u32::MAX {
                                next_frontier.append(col_id as usize, Pattern);
                                self.current_length[col_id as usize] =
                                    self.current_length[row_id] + 1;
                            } else if self.current_length[col_id as usize]
                                > self.current_length[row_id] + 1
                            {
                                self.current_length[col_id as usize] =
                                    self.current_length[row_id] + 1;

                                next_frontier.append(col_id as usize, Pattern);
                            }
                        }
                    }
                    if next_frontier.nnz() == 0 {
                        self.current_frontier = RunningStatus::End;
                        None
                    } else {
                        *frontier = Some(next_frontier.clone());
                        Some(FrontierType::Some(next_frontier))
                    }
                }
                None => {
                    // init frontier
                    let next_frontier = CsVecI::new(self.matrix.cols(), vec![0], vec![Pattern]);
                    *frontier = Some(next_frontier.clone());
                    Some(FrontierType::Some(next_frontier))
                }
            },
            RunningStatus::End => None,
        }
    }
}
