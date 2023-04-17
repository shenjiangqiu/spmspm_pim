use crate::analysis::mapping::{LogicColId, LogicRowId};

use super::EvilFilter;

pub struct AllEvilFilter;

impl EvilFilter for AllEvilFilter {
    fn is_evil_row(&self, _row_id: LogicRowId) -> bool {
        true
    }
    fn is_evil_col(&self, _col_id: LogicColId) -> bool {
        true
    }
}
