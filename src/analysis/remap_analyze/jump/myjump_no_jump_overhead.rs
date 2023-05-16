use serde::{Deserialize, Serialize};

use crate::analysis::remap_analyze::row_cycle::*;

use super::{get_total_row_cycle, AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct MyJumpNoOverhead<const GAP: usize, const WALKER_SIZE: usize> {
    /// the cycle that jump to the target location
    pub multi_jump_cycle: usize,

    /// the cycle that perform stream data read(one jump)
    pub one_jump_cycle: usize,
}
impl<const GAP: usize, const WALKER_SIZE: usize> UpdatableJumpCycle
    for MyJumpNoOverhead<GAP, WALKER_SIZE>
{
    fn update(
        &mut self,
        row_status: &RowIdWordId,
        location: &RowLocation,
        size: WordId,
        _remap_cycle: usize,
    ) {
        let gap = GAP;
        let words_per_waler = WALKER_SIZE / 4;
        // fix the bug here! the ohe is smaller when the WALKER_SIZE is smaller
        let real_loc_word_id = location.row_id_world_id.word_id.0 % words_per_waler;
        let real_row_status_word_id = row_status.word_id.0 % words_per_waler;

        let (first_row, remaining_row) =
            get_total_row_cycle::<WALKER_SIZE>(row_status, location, size);
        let first_row_cycle = first_row * 18;
        let remaining_row_cycle = remaining_row * 18;
        // first find the nearest stop
        let re_map_times = (real_loc_word_id % gap).min(gap - real_loc_word_id % gap);

        let normal_jumps =
            (real_row_status_word_id as isize - real_loc_word_id as isize).unsigned_abs();
        let normal_cycles = (normal_jumps + 6) / 7;
        // need one cycle to jump to the target location
        let remap_cycles = (re_map_times + 6) / 7 + 1;
        let min_jump_cycle = (remap_cycles).min(normal_cycles);
        let min_jump_and_row_cycle = min_jump_cycle.max(first_row_cycle);

        self.multi_jump_cycle += min_jump_and_row_cycle + remaining_row_cycle;

        self.one_jump_cycle += size.0;
    }
}
impl<const GAP: usize, const WALKER_SIZE: usize> JumpCycle for MyJumpNoOverhead<GAP, WALKER_SIZE> {
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

impl<const GAP: usize, const WALKER_SIZE: usize> AddableJumpCycle
    for MyJumpNoOverhead<GAP, WALKER_SIZE>
{
    fn add(&mut self, my_jump_cycle: &MyJumpNoOverhead<GAP, WALKER_SIZE>) {
        self.multi_jump_cycle += my_jump_cycle.multi_jump_cycle;
        self.one_jump_cycle += my_jump_cycle.one_jump_cycle;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_jump() {
        let mut ideal_jump: MyJumpNoOverhead<4, 32> = MyJumpNoOverhead::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_world_id: RowIdWordId {
                row_id: PhysicRowId(0),
                word_id: WordId(0),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(10), 3);
        let total = ideal_jump.total();
        assert_eq!(total, 18 + 10);
    }

    #[test]
    fn test_jump_different_row() {
        let mut ideal_jump: MyJumpNoOverhead<4, 32> = MyJumpNoOverhead::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_world_id: RowIdWordId {
                row_id: PhysicRowId(1),
                word_id: WordId(0),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(10), 3);
        let total = ideal_jump.total();
        assert_eq!(total, 18 + 10 + 18);
    }

    #[test]
    fn test_jump_different_row_with_jump_small() {
        // the gap is 4 words not 4 bytes! so the remap jump is always 1 or 2
        let mut ideal_jump: MyJumpNoOverhead<4, 32> = MyJumpNoOverhead::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_world_id: RowIdWordId {
                row_id: PhysicRowId(1),
                word_id: WordId(6),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(16), 3);
        let total = ideal_jump.total();
        assert_eq!(total, 18 + 16 + 18 + 18);
    }

    #[test]
    fn test_same_row_different_walker() {
        let mut ideal_jump: MyJumpNoOverhead<4, 32> = MyJumpNoOverhead::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_world_id: RowIdWordId {
                row_id: PhysicRowId(0),
                word_id: WordId(8),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(10), 3);
        let total = ideal_jump.total();
        assert_eq!(total, 18 + 10 + 18);
    }

    #[test]
    fn test_same_row_large_jump() {
        let mut ideal_jump: MyJumpNoOverhead<4, 32> = MyJumpNoOverhead::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_world_id: RowIdWordId {
                row_id: PhysicRowId(0),
                word_id: WordId(7),
            },

            subarray_id: SubarrayId(0),
        };
        // no need to jump, the extra size is 1*18=18
        ideal_jump.update(&row_status, &location, WordId(10), 3);
        let total = ideal_jump.total();
        assert_eq!(total, 10 + 18 + 18 + 1);
    }
}
