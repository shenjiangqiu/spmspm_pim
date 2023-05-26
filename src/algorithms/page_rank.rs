use super::{FrontierType, SpmvAlgorithm};

pub struct PageRank;

impl SpmvAlgorithm for PageRank {
    fn next_frontier(&mut self) -> Option<FrontierType> {
        Some(FrontierType::All)
    }
}

#[cfg(test)]
mod tests {
    use super::PageRank;
    use crate::algorithms::SpmvAlgorithm;

    #[test]
    fn test_next_frontier() {
        let mut page_rank = PageRank;
        assert_eq!(page_rank.next_frontier(), Some(super::FrontierType::All));
    }
}
