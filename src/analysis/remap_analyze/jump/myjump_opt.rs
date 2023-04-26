use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::{AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

/// the optimized jump cycle, the normal jump and the calculation is overlapped
#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct MyJumpOpt<const GAP: usize> {
    /// the cycle that jump to the target location
    pub multi_jump_cycle: usize,

    /// the cycle that perform stream data read(one jump)
    pub one_jump_cycle: usize,
}
impl<const GAP: usize> UpdatableJumpCycle for MyJumpOpt<GAP> {
    fn update(
        &mut self,
        row_status: &(usize, usize),
        loc: &RowLocation,
        size: usize,
        remap_unit: usize,
    ) {
        let gap = GAP;
        let row_cycle = if loc.row_id.0 == row_status.0 { 0 } else { 18 };

        // first find the nearest stop
        let re_map_times = (loc.col_id.0 % gap).min(gap - loc.col_id.0 % gap);

        let from_start_cycle = loc.col_id.0;
        let normal_cycle = (row_status.1 as isize - loc.col_id.0 as isize).abs() as usize;
        let min_jump_cycle = (re_map_times + 1 + remap_unit)
            .min(from_start_cycle + 1)
            .min(normal_cycle);

        let min_jump_and_row_cycle = min_jump_cycle.max(row_cycle);

        self.multi_jump_cycle += min_jump_and_row_cycle;

        self.one_jump_cycle += size * 4;
    }
}
impl<const GAP: usize> JumpCycle for MyJumpOpt<GAP> {
    fn total(&self) -> usize {
        self.multi_jump_cycle + self.one_jump_cycle
    }

    fn get_one_jump(&self) -> usize {
        self.one_jump_cycle
    }
    fn get_one_jump_mut(&mut self) -> &mut usize {
        &mut self.one_jump_cycle
    }

    fn get_multi_jump(&self) -> usize {
        self.multi_jump_cycle
    }
    fn get_multi_jump_mut(&mut self) -> &mut usize {
        &mut self.multi_jump_cycle
    }
}

impl<const GAP: usize> AddableJumpCycle for MyJumpOpt<GAP> {
    fn add(&mut self, other: &Self) {
        self.multi_jump_cycle += other.multi_jump_cycle;
        self.one_jump_cycle += other.one_jump_cycle;
    }
}
