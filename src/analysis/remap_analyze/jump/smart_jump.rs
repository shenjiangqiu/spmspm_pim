use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::{AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct SmartJumpCycle {
    pub jump_one_cycle: usize,
    pub jump_multiple_cycle: usize,
}
impl UpdatableJumpCycle for SmartJumpCycle {
    fn update(
        &mut self,
        row_status: &(usize, usize),
        location: &RowLocation,
        size: usize,
        _remap_cycle: usize,
    ) {
        let row_cycle = if location.row_id.0 == row_status.0 {
            0
        } else {
            18
        };
        let current_col = row_status.1;
        let target_col = location.col_id.0;
        let jumps = (current_col as isize - target_col as isize).abs() as usize;
        let jumps = jumps.min(target_col + 1);
        // the jump of size
        if jumps > 4 {
            self.jump_multiple_cycle += jumps.max(row_cycle);
        } else {
            self.jump_one_cycle += jumps.max(row_cycle);
        }
        self.jump_one_cycle += size * 4;
    }
}
impl JumpCycle for SmartJumpCycle {
    fn total(&self) -> usize {
        self.jump_multiple_cycle + self.jump_one_cycle
    }

    fn get_one_jump(&self) -> usize {
        self.jump_one_cycle
    }

    fn get_multi_jump(&self) -> usize {
        self.jump_multiple_cycle
    }

    fn get_one_jump_mut(&mut self) -> &mut usize {
        &mut self.jump_one_cycle
    }

    fn get_multi_jump_mut(&mut self) -> &mut usize {
        &mut self.jump_multiple_cycle
    }
}

impl AddableJumpCycle for SmartJumpCycle {
    fn add(&mut self, smart_jump_cycle: &SmartJumpCycle) {
        self.jump_one_cycle += smart_jump_cycle.jump_one_cycle;
        self.jump_multiple_cycle += smart_jump_cycle.jump_multiple_cycle;
    }
}
