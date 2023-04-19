use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::JumpCycle;

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct NormalJumpCycle {
    pub jump_one_cycle: usize,
    pub jump_multiple_cycle: usize,
}
impl NormalJumpCycle {
    pub fn update(&mut self, evil_row_status: (usize, usize), location: &RowLocation, size: usize) {
        let current_col = evil_row_status.1;
        let target_col = location.col_id.0;
        let jumps = (current_col as isize - target_col as isize).abs() as usize;
        // the jump of size
        if jumps > 4 {
            self.jump_multiple_cycle += jumps;
        } else {
            self.jump_one_cycle += jumps;
        }
        self.jump_one_cycle += size * 4;
    }
}
impl JumpCycle for NormalJumpCycle {
    fn total(&self) -> usize {
        self.jump_multiple_cycle + self.jump_one_cycle
    }

    fn add(&mut self, normal_jump_cycle: &NormalJumpCycle) {
        self.jump_one_cycle += normal_jump_cycle.jump_one_cycle;
        self.jump_multiple_cycle += normal_jump_cycle.jump_multiple_cycle;
    }
}
