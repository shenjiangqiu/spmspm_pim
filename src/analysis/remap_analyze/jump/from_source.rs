use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::JumpCycle;

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct FromSourceJumpCycle {
    pub jump_one_cycle: usize,
    pub jump_multiple_cycle: usize,
}
impl FromSourceJumpCycle {
    pub fn update(&mut self, _row_status: &(usize, usize), location: &RowLocation, size: usize) {
        let row_cycle = if location.row_id.0 == _row_status.0 {
            0
        } else {
            18
        };

        if location.col_id.0 > 4 {
            self.jump_multiple_cycle += (location.col_id.0 + 1).max(row_cycle);
        } else {
            self.jump_one_cycle += (location.col_id.0 + 1).max(row_cycle);
        }
        self.jump_one_cycle += size * 4;
    }
}
impl JumpCycle for FromSourceJumpCycle {
    fn add(&mut self, from_source_jump_cycle: &FromSourceJumpCycle) {
        self.jump_one_cycle += from_source_jump_cycle.jump_one_cycle;
        self.jump_multiple_cycle += from_source_jump_cycle.jump_multiple_cycle;
    }

    fn total(&self) -> usize {
        self.jump_one_cycle + self.jump_multiple_cycle
    }
}
