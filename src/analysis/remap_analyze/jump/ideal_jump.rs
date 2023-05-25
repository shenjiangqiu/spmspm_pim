use serde::{Deserialize, Serialize};

use crate::analysis::remap_analyze::row_cycle::*;

use super::{get_total_row_cycle, AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct IdealJumpCycle<const WALKER_SIZE: usize> {
    pub total_cycle: usize,
}
impl<const WALKER_SIZE: usize> UpdatableJumpCycle for IdealJumpCycle<WALKER_SIZE> {
    fn update(
        &mut self,
        row_status: &RowIdWordId,
        loc: &RowLocation,
        size: WordId,
        _remap_cycle: usize,
    ) {
        let words_per_waler = WALKER_SIZE / 4;
        // fix the bug here! the ohe is smaller when the WALKER_SIZE is smaller
        let real_loc_word_id = loc.row_id_word_id.word_id.0 % words_per_waler;
        let real_row_status_word_id = row_status.word_id.0 % words_per_waler;

        let (first_row, remaining_row) = get_total_row_cycle::<WALKER_SIZE>(row_status, loc, size);

        if real_loc_word_id != real_row_status_word_id {
            // it' not the same col
            self.total_cycle += 1.max(first_row * 18);
        } else {
            self.total_cycle += first_row * 18;
        }
        self.total_cycle += size.0 + remaining_row * 18;
    }
}
impl<const WALKER_SIZE: usize> JumpCycle for IdealJumpCycle<WALKER_SIZE> {
    fn total(&self) -> usize {
        self.total_cycle
    }

    fn get_one_jump(&self) -> usize {
        self.total_cycle
    }

    fn get_multi_jump(&self) -> usize {
        0
    }
    fn get_multi_jump_mut(&mut self) -> &mut usize {
        &mut self.total_cycle
    }
    fn get_one_jump_mut(&mut self) -> &mut usize {
        &mut self.total_cycle
    }
}

impl<const WALKER_SIZE: usize> AddableJumpCycle for IdealJumpCycle<WALKER_SIZE> {
    fn add(&mut self, ideal_jump_cycle: &IdealJumpCycle<WALKER_SIZE>) {
        self.total_cycle += ideal_jump_cycle.total_cycle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ideal_jump() {
        let mut ideal_jump: IdealJumpCycle<32> = IdealJumpCycle::default();
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
        assert_eq!(total, 18 + 10);
    }

    #[test]
    fn test_ideal_jump_different_row() {
        let mut ideal_jump: IdealJumpCycle<32> = IdealJumpCycle::default();
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
        assert_eq!(total, 18 + 10 + 18);
    }

    #[test]
    fn test_same_row_different_walker() {
        let mut ideal_jump: IdealJumpCycle<32> = IdealJumpCycle::default();
        let row_status = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let location = RowLocation {
            row_id_word_id: RowIdWordId {
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
}
