use super::{FrontierType, SpmvAlgorithm};

pub struct PageRank;

impl SpmvAlgorithm for PageRank {
    fn next_frontier(&mut self) -> Option<FrontierType> {
        Some(FrontierType::All)
    }
}
