use crate::analysis::mapping::{LogicColId, LogicRowId};

use super::EvilFilter;

pub struct NoEvilFilter;

impl EvilFilter for NoEvilFilter {
    fn is_evil_row(&self, _row_id: LogicRowId) -> bool {
        false
    }
    fn is_evil_col(&self, _col_id: LogicColId) -> bool {
        false
    }
}
