use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::{AddableJumpCycle, JumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct NormalJumpCycle {
    pub jump_one_cycle: usize,
    pub jump_multiple_cycle: usize,

    // the statistics
    pub total_jumps_all: usize,
    pub total_jumps_covered_by_row_open: usize,
    pub jumps_not_covered_when_no_row_open: usize,
    pub jumps_not_covered_when_more_shift: usize,
}
impl NormalJumpCycle {
    pub fn update(
        &mut self,
        evil_row_status: &(usize, usize),
        location: &RowLocation,
        size: usize,
    ) {
        let row_cycle = if location.row_id.0 == evil_row_status.0 {
            0
        } else {
            18
        };
        let current_col = evil_row_status.1;
        let target_col = location.col_id.0;
        let jumps = (current_col as isize - target_col as isize).abs() as usize;
        // update the statistics
        self.total_jumps_all += jumps;
        if jumps <= row_cycle {
            self.total_jumps_covered_by_row_open += jumps;
        } else {
            // cannot cover by row open
            if jumps == 0 {
                self.jumps_not_covered_when_no_row_open += jumps;
            } else {
                self.jumps_not_covered_when_more_shift += jumps - row_cycle;
            }
        }
        // the jump of size
        if jumps > 4 {
            self.jump_multiple_cycle += jumps.max(row_cycle);
        } else {
            self.jump_one_cycle += jumps.max(row_cycle);
        }
        self.jump_one_cycle += size * 4;
    }
}
impl JumpCycle for NormalJumpCycle {
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
impl AddableJumpCycle for NormalJumpCycle {
    fn add(&mut self, normal_jump_cycle: &NormalJumpCycle) {
        self.jump_one_cycle += normal_jump_cycle.jump_one_cycle;
        self.jump_multiple_cycle += normal_jump_cycle.jump_multiple_cycle;
        self.total_jumps_all += normal_jump_cycle.total_jumps_all;
        self.total_jumps_covered_by_row_open += normal_jump_cycle.total_jumps_covered_by_row_open;
        self.jumps_not_covered_when_no_row_open +=
            normal_jump_cycle.jumps_not_covered_when_no_row_open;
        self.jumps_not_covered_when_more_shift +=
            normal_jump_cycle.jumps_not_covered_when_more_shift;
    }
}
