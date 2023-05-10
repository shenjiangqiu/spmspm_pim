use crate::analysis::remap_analyze::row_cycle::*;

use super::EvilFilter;

pub struct PercentEvilFilter {
    row_evil_threshold: usize,
    col_evil_threshold: usize,
}

impl PercentEvilFilter {
    pub fn new(row_evil_threshold: usize, col_evil_threshold: usize) -> Self {
        Self {
            row_evil_threshold,
            col_evil_threshold,
        }
    }
}

impl EvilFilter for PercentEvilFilter {
    fn is_evil_row(&self, row_id: LogicRowId) -> bool {
        let row_id = row_id.0;
        row_id < self.row_evil_threshold
    }
    fn is_evil_col(&self, col_id: LogicColId) -> bool {
        let col_id = col_id.0;
        col_id < self.col_evil_threshold
    }
}
