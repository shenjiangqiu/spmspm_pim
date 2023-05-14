use serde::{Deserialize, Serialize};

use crate::analysis::remap_analyze::row_cycle::*;

use super::{get_total_row_cycle, AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

/// only jump, not compare and shifting
#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct MyJumpOnly<const GAP: usize, const WALKER_SIZE: usize> {
    /// the cycle to calculate the remap location(0xgap or 1xgap...)
    pub calculate_remap_cycle: usize,

    /// the cycle that jump to the target location
    pub multi_jump_cycle: usize,

    /// the cycle that perform stream data read(one jump)
    pub one_jump_cycle: usize,
}
impl<const GAP: usize, const WALKER_SIZE: usize> UpdatableJumpCycle
    for MyJumpOnly<GAP, WALKER_SIZE>
{
    fn update(
        &mut self,
        row_status: &RowIdWordId,
        loc: &RowLocation,
        size: WordId,
        remap_unit: usize,
    ) {
        let gap = GAP;
        let row_cycle = get_total_row_cycle::<WALKER_SIZE>(row_status, loc, size);

        self.calculate_remap_cycle += remap_unit;

        // first find the nearest stop
        let re_map_times =
            (loc.row_id_world_id.word_id.0 % gap).min(gap - loc.row_id_world_id.word_id.0 % gap);

        // let normal_cycle = (row_status.1 .0 as isize - loc.word_id.0 as isize).abs() as usize;
        let min_jump_cycle = re_map_times + 1;
        let min_jump_cycle = (min_jump_cycle + 6) / 7;

        let min_jump_and_row_cycle = min_jump_cycle.max(row_cycle);

        self.multi_jump_cycle += min_jump_and_row_cycle;
        self.one_jump_cycle += size.0;
    }
}
impl<const GAP: usize, const WALKER_SIZE: usize> JumpCycle for MyJumpOnly<GAP, WALKER_SIZE> {
    fn total(&self) -> usize {
        self.calculate_remap_cycle + self.multi_jump_cycle + self.one_jump_cycle
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
impl<const GAP: usize, const WALKER_SIZE: usize> AddableJumpCycle for MyJumpOnly<GAP, WALKER_SIZE> {
    fn add(&mut self, other: &Self) {
        self.calculate_remap_cycle += other.calculate_remap_cycle;
        self.multi_jump_cycle += other.multi_jump_cycle;
        self.one_jump_cycle += other.one_jump_cycle;
    }
}
