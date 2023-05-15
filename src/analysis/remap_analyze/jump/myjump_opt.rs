use serde::{Deserialize, Serialize};

use crate::analysis::remap_analyze::row_cycle::*;

use super::{
    get_num_extra_walkers_to_load, get_total_row_cycle, AddableJumpCycle, JumpCycle,
    UpdatableJumpCycle,
};

/// the optimized jump cycle, the normal jump and the calculation is overlapped
#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct MyJumpOpt<const GAP: usize, const WALKER_SIZE: usize> {
    /// the cycle that jump to the target location
    pub multi_jump_cycle: usize,

    /// the cycle that perform stream data read(one jump)
    pub one_jump_cycle: usize,
    /// the row open cycle for the request
    pub row_cycle_total: usize,
    pub total_accesses: usize,
    pub row_hits: usize,
    pub row_misses: usize,
    // global row for all sequential accesses
    pub gloabl_row_accesses: usize,
    pub global_row_hits: usize,
    pub global_row_miss: usize,
    pub global_row_cycles: usize,

    /// histogram
    pub opt_saved_times: usize,
    pub opt_saved_cycles: usize,
    pub all_cycle_hist_0: usize,
    pub all_cycle_hist_1_2: usize,
    pub all_cycle_hist_3_4: usize,
    pub all_cycle_hist_5_8: usize,
    pub all_cycle_hist_9_and_more: usize,
}
impl<const GAP: usize, const WALKER_SIZE: usize> UpdatableJumpCycle
    for MyJumpOpt<GAP, WALKER_SIZE>
{
    fn update(
        &mut self,
        row_status: &RowIdWordId,
        loc: &RowLocation,
        size: WordId,
        remap_unit: usize,
    ) {
        let gap = GAP;
        self.total_accesses += 1;
        self.gloabl_row_accesses += 1;
        if row_status.row_id != loc.row_id_world_id.row_id {
            self.row_misses += 1;
            self.global_row_miss += 1;
            self.row_cycle_total += 18;
            self.global_row_cycles += 18;
        } else {
            self.row_hits += 1;
            self.global_row_hits += 1;
        }

        let (first_row, remaining_row) = get_total_row_cycle::<WALKER_SIZE>(row_status, loc, size);
        let first_row_cycle = first_row * 18;
        let remaining_row_cycle = remaining_row * 18;
        let extra_walkers_to_read =
            get_num_extra_walkers_to_load::<WALKER_SIZE>(loc.row_id_world_id.word_id, size);
        self.gloabl_row_accesses += extra_walkers_to_read;
        self.global_row_miss += extra_walkers_to_read;
        self.global_row_cycles += extra_walkers_to_read * 18;

        // first find the nearest stop
        let re_map_times =
            (loc.row_id_world_id.word_id.0 % gap).min(gap - loc.row_id_world_id.word_id.0 % gap);

        let normal_cycle =
            (row_status.word_id.0 as isize - loc.row_id_world_id.word_id.0 as isize).abs() as usize;

        let min_jump_cycle = (re_map_times + 1 + remap_unit).min(normal_cycle);
        let min_jump_cycle = (min_jump_cycle + 6) / 7;

        let min_jump_and_row_cycle = min_jump_cycle.max(first_row_cycle);

        self.multi_jump_cycle += min_jump_and_row_cycle + remaining_row_cycle;

        self.one_jump_cycle += size.0;

        // update the histogram
        if min_jump_cycle < (re_map_times + 1 + remap_unit) {
            // my jump is not used so the time is saved
            self.opt_saved_times += 1;

            let new_min_jump_and_row_cycle = min_jump_cycle.max(first_row_cycle);
            let saved_cycle = new_min_jump_and_row_cycle - min_jump_and_row_cycle;
            debug_assert!(new_min_jump_and_row_cycle >= min_jump_and_row_cycle);
            self.opt_saved_cycles += saved_cycle;
        }
        // update the histogram
        match normal_cycle {
            0 => self.all_cycle_hist_0 += 1,
            1..=2 => self.all_cycle_hist_1_2 += 1,
            3..=4 => self.all_cycle_hist_3_4 += 1,
            5..=8 => self.all_cycle_hist_5_8 += 1,
            _ => self.all_cycle_hist_9_and_more += 1,
        }
    }
}

impl<const GAP: usize, const WALKER_SIZE: usize> JumpCycle for MyJumpOpt<GAP, WALKER_SIZE> {
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

impl<const GAP: usize, const WALKER_SIZE: usize> AddableJumpCycle for MyJumpOpt<GAP, WALKER_SIZE> {
    fn add(&mut self, other: &Self) {
        self.multi_jump_cycle += other.multi_jump_cycle;
        self.one_jump_cycle += other.one_jump_cycle;

        self.opt_saved_times += other.opt_saved_times;
        self.opt_saved_cycles += other.opt_saved_cycles;
        self.all_cycle_hist_0 += other.all_cycle_hist_0;
        self.all_cycle_hist_1_2 += other.all_cycle_hist_1_2;
        self.all_cycle_hist_3_4 += other.all_cycle_hist_3_4;
        self.all_cycle_hist_5_8 += other.all_cycle_hist_5_8;
        self.all_cycle_hist_9_and_more += other.all_cycle_hist_9_and_more;
    }
}
