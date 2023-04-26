use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::{AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct MyJumpNoOverhead<const GAP: usize> {
    /// the cycle that jump to the target location
    pub multi_jump_cycle: usize,

    /// the cycle that perform stream data read(one jump)
    pub one_jump_cycle: usize,
}
impl<const GAP: usize> UpdatableJumpCycle for MyJumpNoOverhead<GAP> {
    fn update(
        &mut self,
        row_status: &(usize, usize),
        location: &RowLocation,
        size: usize,
        _remap_cycle: usize,
    ) {
        let gap = GAP;
        let row_cycle = if location.row_id.0 == row_status.0 {
            0
        } else {
            18
        };

        // first find the nearest stop
        let re_map_times = (location.col_id.0 % gap).min(gap - location.col_id.0 % gap);

        let from_start_cycle = location.col_id.0;
        let normal_cycle = (row_status.1 as isize - location.col_id.0 as isize).abs() as usize;
        let min_jump_cycle = (re_map_times + 1)
            .min(from_start_cycle + 1)
            .min(normal_cycle);
        let min_jump_and_row_cycle = min_jump_cycle.max(row_cycle);

        self.multi_jump_cycle += min_jump_and_row_cycle;

        self.one_jump_cycle += size * 4;
    }
}
impl<const GAP: usize> JumpCycle for MyJumpNoOverhead<GAP> {
    fn total(&self) -> usize {
        self.multi_jump_cycle + self.one_jump_cycle
    }

    fn get_one_jump(&self) -> usize {
        self.one_jump_cycle
    }

    fn get_multi_jump(&self) -> usize {
        self.multi_jump_cycle
    }

    fn get_one_jump_mut(&mut self) -> &mut usize {
        &mut self.one_jump_cycle
    }

    fn get_multi_jump_mut(&mut self) -> &mut usize {
        &mut self.multi_jump_cycle
    }
}

impl<const GAP: usize> AddableJumpCycle for MyJumpNoOverhead<GAP> {
    fn add(&mut self, my_jump_cycle: &MyJumpNoOverhead<GAP>) {
        self.multi_jump_cycle += my_jump_cycle.multi_jump_cycle;
        self.one_jump_cycle += my_jump_cycle.one_jump_cycle;
    }
}
