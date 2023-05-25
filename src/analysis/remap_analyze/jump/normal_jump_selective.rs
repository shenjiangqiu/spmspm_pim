use serde::{Deserialize, Serialize};

use crate::analysis::remap_analyze::{
    remote_updator::{selective::SelectiveUpdator, RemoteUpdator},
    row_cycle::*,
};

use super::{get_total_row_cycle, AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct NormalJumpCycleSelective<const WALKER_SIZE: usize> {
    pub jump_one_cycle: usize,
    pub jump_multiple_cycle: usize,

    // the statistics
    pub total_jumps_all: usize,
    pub total_jumps_covered_by_row_open: usize,
    pub jumps_not_covered_when_no_row_open: usize,
    pub jumps_not_covered_when_more_shift: usize,
    pub extra_scan_cycles: usize,
}
impl<const WALKER_SIZE: usize> UpdatableJumpCycle for NormalJumpCycleSelective<WALKER_SIZE> {
    fn update(
        &mut self,
        row_status: &RowIdWordId,
        location: &RowLocation,
        size: WordId,
        _remap_cycle: usize,
    ) {
        // fix the bug here,
        let words_per_waler = WALKER_SIZE / 4;
        // fix the bug here! the ohe is smaller when the WALKER_SIZE is smaller
        let real_loc_word_id = location.row_id_word_id.word_id.0 % words_per_waler;
        let real_row_status_word_id = row_status.word_id.0 % words_per_waler;

        let (first_row, remaining_row) =
            get_total_row_cycle::<WALKER_SIZE>(row_status, location, size);
        let first_row_cycle = first_row * 18;
        let remaining_row_cycle = remaining_row * 18;
        let jumps: usize =
            (real_loc_word_id as isize - real_row_status_word_id as isize).unsigned_abs();
        let jumps = (jumps + 6) / 7;
        // update the statistics
        // fix bug here, should add the coverd when not totally covered
        self.total_jumps_all += jumps;
        if jumps <= first_row_cycle {
            self.total_jumps_covered_by_row_open += jumps;
        } else {
            // cannot cover by row open
            // fix bug here, it's rowcycle not jumps!!!
            if first_row == 0 {
                // cannot cover by
                self.jumps_not_covered_when_no_row_open += jumps;
            } else {
                self.jumps_not_covered_when_more_shift += jumps - first_row_cycle;
                self.total_jumps_covered_by_row_open += first_row_cycle;
            }
        }

        // the jump of size
        if jumps > 1 {
            self.jump_multiple_cycle += jumps.max(first_row_cycle) + remaining_row_cycle;
        } else {
            self.jump_one_cycle += jumps.max(first_row_cycle) + remaining_row_cycle;
        }
        self.jump_one_cycle += size.0;
    }

    fn batch_update(
        &mut self,
        row_status: &RowIdWordId,
        loc: &[RowLocation],
        size: WordId,
        remap_cycle: usize,
    ) {
        let mut selective_updator =
            SelectiveUpdator::<WALKER_SIZE, _>::new(self, *row_status, size, remap_cycle);
        selective_updator.update(loc);
        let extra_cycle = selective_updator.extra_scan_cycles;
        self.extra_scan_cycles += extra_cycle;
    }
}
impl<const WALKER_SIZE: usize> NormalJumpCycleSelective<WALKER_SIZE> {
    /// the rate of jumps that can be covered by row open, jumps that cannot be covered by row open when no row open, jumps that cannot be covered by row open when more shift
    pub fn cover_rate(&self) -> [f32; 3] {
        [
            self.total_jumps_covered_by_row_open as f32 / self.total_jumps_all as f32,
            self.jumps_not_covered_when_no_row_open as f32 / self.total_jumps_all as f32,
            self.jumps_not_covered_when_more_shift as f32 / self.total_jumps_all as f32,
        ]
    }
}
impl<const WALKER_SIZE: usize> JumpCycle for NormalJumpCycleSelective<WALKER_SIZE> {
    fn total(&self) -> usize {
        self.jump_multiple_cycle + self.jump_one_cycle + self.extra_scan_cycles
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
impl<const WALKER_SIZE: usize> AddableJumpCycle for NormalJumpCycleSelective<WALKER_SIZE> {
    fn add(&mut self, normal_jump_cycle: &NormalJumpCycleSelective<WALKER_SIZE>) {
        self.jump_one_cycle += normal_jump_cycle.jump_one_cycle;
        self.jump_multiple_cycle += normal_jump_cycle.jump_multiple_cycle;
        self.extra_scan_cycles += normal_jump_cycle.extra_scan_cycles;

        self.total_jumps_all += normal_jump_cycle.total_jumps_all;
        self.total_jumps_covered_by_row_open += normal_jump_cycle.total_jumps_covered_by_row_open;
        self.jumps_not_covered_when_no_row_open +=
            normal_jump_cycle.jumps_not_covered_when_no_row_open;
        self.jumps_not_covered_when_more_shift +=
            normal_jump_cycle.jumps_not_covered_when_more_shift;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_jump() {
        let mut ideal_jump: NormalJumpCycleSelective<32> = NormalJumpCycleSelective::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_word_id: RowIdWordId {
                row_id: PhysicRowId(0),
                word_id: WordId(0),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(10), 3);
        let total = ideal_jump.total();
        // 0 first row, 1 extra row, 10 words, 3 cal, 1 jump(0-0)no cover
        assert_eq!(total, 18 + 10);
    }

    #[test]
    fn test_jump_different_row() {
        let mut ideal_jump: NormalJumpCycleSelective<32> = NormalJumpCycleSelective::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_word_id: RowIdWordId {
                row_id: PhysicRowId(1),
                word_id: WordId(0),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(10), 3);
        let total = ideal_jump.total();
        // 1 first row, 1 extra row, 10 words, 3 cal, 1 jump(0-0) covererd by first row
        assert_eq!(total, 18 + 10 + 18);
    }

    #[test]
    fn test_jump_different_row_with_jump_small() {
        // the gap is 4 words not 4 bytes! so the remap jump is always 1 or 2
        let mut ideal_jump: NormalJumpCycleSelective<32> = NormalJumpCycleSelective::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_word_id: RowIdWordId {
                row_id: PhysicRowId(1),
                word_id: WordId(6),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(16), 3);
        let total = ideal_jump.total();
        // 1 first row, 2 extra row, 16 words, 3 cal, 2 jump(0-4-6) covererd by first row
        assert_eq!(total, 18 + 16 + 18 + 18);
    }
    #[test]
    fn test_jump_different_row_with_jump_large() {
        // the gap is 4 words not 4 bytes! so the remap jump is always 1 or 2
        let mut ideal_jump: NormalJumpCycleSelective<32> = NormalJumpCycleSelective::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_word_id: RowIdWordId {
                row_id: PhysicRowId(1),
                word_id: WordId(16),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        // 1 first row, 1 extra row, 16 words, 3 cal, 1 jump(0-16) covererd by first row
        ideal_jump.update(&row_status, &location, WordId(16), 3);
        let total = ideal_jump.total();
        assert_eq!(total, 18 + 16 + 18);
    }
}
