use serde::{Deserialize, Serialize};

use crate::analysis::translate_mapping::RowLocation;

use super::JumpCycle;

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct MyJumpCycle {
    /// each time calculate the remap location need this cycle
    pub remap_unit: usize,
    /// the cycle to calculate the remap location(0xgap or 1xgap...)
    pub calculate_remap_cycle: usize,
    /// the cycle that jump to the target location
    pub multi_jump_cycle: usize,
    /// the times that jump to the remap location
    pub remap_jump_times: usize,
    /// the cycle that perform stream data read(one jump)
    pub one_jump_cycle: usize,
    /// the gaps of each stops in the one-hot vector which can be quickly jump in
    pub gap: usize,
}
impl MyJumpCycle {
    pub fn new(remap_unit: usize, gap: usize) -> Self {
        Self {
            remap_unit,
            gap,
            ..Default::default()
        }
    }
    pub fn update(&mut self, evil_row_status: (usize, usize), location: &RowLocation, size: usize) {
        self.calculate_remap_cycle += self.remap_unit;
        // first find the nearest stop
        let re_map_times =
            (location.col_id.0 % self.gap).min(self.gap - location.col_id.0 % self.gap);
        let from_start_cycle = location.col_id.0;
        let normal_cycle = (evil_row_status.1 as isize - location.col_id.0 as isize).abs() as usize;
        let min = (re_map_times + 1)
            .min(from_start_cycle + 1)
            .min(normal_cycle);
        if min == re_map_times + 1 {
            self.remap_jump_times += 1;
            self.multi_jump_cycle += re_map_times;
        } else if min == from_start_cycle + 1 {
            self.multi_jump_cycle += from_start_cycle + 1;
        } else {
            self.multi_jump_cycle += normal_cycle;
        }

        self.one_jump_cycle += size * 4;
    }
}
impl JumpCycle for MyJumpCycle {
    fn add(&mut self, my_jump_cycle: &MyJumpCycle) {
        self.calculate_remap_cycle += my_jump_cycle.calculate_remap_cycle;
        self.multi_jump_cycle += my_jump_cycle.multi_jump_cycle;
        self.remap_jump_times += my_jump_cycle.remap_jump_times;
        self.one_jump_cycle += my_jump_cycle.one_jump_cycle;
    }

    fn total(&self) -> usize {
        self.calculate_remap_cycle
            + self.multi_jump_cycle
            + self.one_jump_cycle
            + self.remap_jump_times
    }
}
