use serde::{Deserialize, Serialize};

use crate::analysis::{
    mapping::{PhysicRowId, WordId},
    translate_mapping::RowLocation,
};

use super::{AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct MyJumpCycle<const GAP: usize> {
    /// the cycle to calculate the remap location(0xgap or 1xgap...)
    pub calculate_remap_cycle: usize,

    /// the cycle that jump to the target location
    pub multi_jump_cycle: usize,

    /// the cycle that perform stream data read(one jump)
    pub one_jump_cycle: usize,
}
impl<const GAP: usize> UpdatableJumpCycle for MyJumpCycle<GAP> {
    fn update(
        &mut self,
        row_status: &(PhysicRowId, WordId),
        loc: &RowLocation,
        size: WordId,
        remap_unit: usize,
    ) {
        let gap = GAP;
        let row_cycle = if loc.row_id == row_status.0 { 0 } else { 18 };
        self.calculate_remap_cycle += remap_unit;

        // first find the nearest stop
        let re_map_times = (loc.word_id.0 % gap).min(gap - loc.word_id.0 % gap);

        let normal_cycle = (row_status.1 .0 as isize - loc.word_id.0 as isize).abs() as usize;
        let min_jump_cycle = (re_map_times + 1).min(normal_cycle);
        let min_jump_and_row_cycle = min_jump_cycle.max(row_cycle);

        self.multi_jump_cycle += min_jump_and_row_cycle;

        self.one_jump_cycle += size.0;
    }
}
impl<const GAP: usize> JumpCycle for MyJumpCycle<GAP> {
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
impl<const GAP: usize> AddableJumpCycle for MyJumpCycle<GAP> {
    fn add(&mut self, other: &Self) {
        self.calculate_remap_cycle += other.calculate_remap_cycle;
        self.multi_jump_cycle += other.multi_jump_cycle;
        self.one_jump_cycle += other.one_jump_cycle;
    }
}
