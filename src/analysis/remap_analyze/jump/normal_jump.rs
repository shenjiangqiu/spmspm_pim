use serde::{Deserialize, Serialize};

use crate::analysis::remap_analyze::row_cycle::*;

use super::{check_same_walker, AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct NormalJumpCycle<const WALKER_SIZE: usize> {
    pub jump_one_cycle: usize,
    pub jump_multiple_cycle: usize,

    // the statistics
    pub total_jumps_all: usize,
    pub total_jumps_covered_by_row_open: usize,
    pub jumps_not_covered_when_no_row_open: usize,
    pub jumps_not_covered_when_more_shift: usize,
}
impl<const WALKER_SIZE: usize> UpdatableJumpCycle for NormalJumpCycle<WALKER_SIZE> {
    fn update(
        &mut self,
        evil_row_status: &RowIdWordId,
        location: &RowLocation,
        size: WordId,
        _remap_cycle: usize,
    ) {
        // fix the bug here,
        let row_cycle =
            if check_same_walker::<WALKER_SIZE>(evil_row_status, &location.row_id_world_id) {
                0
            } else {
                18
            };
        let jumps = (location.row_id_world_id.word_id.0 as isize
            - evil_row_status.word_id.0 as isize)
            .abs() as usize;
        let jumps = (jumps + 6) / 7;
        // update the statistics
        // fix bug here, should add the coverd when not totally covered
        self.total_jumps_all += jumps;
        if jumps <= row_cycle {
            self.total_jumps_covered_by_row_open += jumps;
        } else {
            // cannot cover by row open
            // fix bug here, it's rowcycle not jumps!!!
            if row_cycle == 0 {
                // cannot cover by
                self.jumps_not_covered_when_no_row_open += jumps;
            } else {
                self.jumps_not_covered_when_more_shift += jumps - row_cycle;
                self.total_jumps_covered_by_row_open += row_cycle;
            }
        }

        // the jump of size
        if jumps > 1 {
            self.jump_multiple_cycle += jumps.max(row_cycle);
        } else {
            self.jump_one_cycle += jumps.max(row_cycle);
        }
        self.jump_one_cycle += size.0;
    }
}
impl<const WALKER_SIZE: usize> NormalJumpCycle<WALKER_SIZE> {
    /// the rate of jumps that can be covered by row open, jumps that cannot be covered by row open when no row open, jumps that cannot be covered by row open when more shift
    pub fn cover_rate(&self) -> [f32; 3] {
        [
            self.total_jumps_covered_by_row_open as f32 / self.total_jumps_all as f32,
            self.jumps_not_covered_when_no_row_open as f32 / self.total_jumps_all as f32,
            self.jumps_not_covered_when_more_shift as f32 / self.total_jumps_all as f32,
        ]
    }
}
impl<const WALKER_SIZE: usize> JumpCycle for NormalJumpCycle<WALKER_SIZE> {
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
impl<const WALKER_SIZE: usize> AddableJumpCycle for NormalJumpCycle<WALKER_SIZE> {
    fn add(&mut self, normal_jump_cycle: &NormalJumpCycle<WALKER_SIZE>) {
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
