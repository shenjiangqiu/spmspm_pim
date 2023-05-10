pub mod all_evil;
pub mod no_evil;

use crate::analysis::remap_analyze::row_cycle::*;

pub mod percent_evil_filter;
/// the trait for filter evil rows and evil columns
pub trait EvilFilter {
    fn is_evil_row(&self, row_id: LogicRowId) -> bool;
    fn is_evil_col(&self, col_id: LogicColId) -> bool;
}
