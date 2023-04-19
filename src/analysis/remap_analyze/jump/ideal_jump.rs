use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::JumpCycle;

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct IdealJumpCycle {
    pub total_cycle: usize,
}
impl IdealJumpCycle {
    pub fn update(
        &mut self,
        _evil_row_status: (usize, usize),
        _location: &RowLocation,
        size: usize,
    ) {
        if _location.col_id.0 != _evil_row_status.1 {
            // it' not the same col
            self.total_cycle += 1;
        }
        self.total_cycle += size * 4;
    }
}
impl JumpCycle for IdealJumpCycle {
    fn add(&mut self, ideal_jump_cycle: &IdealJumpCycle) {
        self.total_cycle += ideal_jump_cycle.total_cycle;
    }

    fn total(&self) -> usize {
        self.total_cycle
    }
}
