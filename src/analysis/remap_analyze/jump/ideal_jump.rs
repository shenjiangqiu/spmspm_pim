use serde::{Deserialize, Serialize};

use crate::analysis::{
    mapping::{PhysicRowId, WordId},
    translate_mapping::RowLocation,
};

use super::{AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct IdealJumpCycle {
    pub total_cycle: usize,
}
impl UpdatableJumpCycle for IdealJumpCycle {
    fn update(
        &mut self,
        row_status: &(PhysicRowId, WordId),
        loc: &RowLocation,
        size: WordId,
        _remap_cycle: usize,
    ) {
        let row_cycle = if loc.row_id == row_status.0 { 0 } else { 18 };
        if loc.word_id != row_status.1 {
            // it' not the same col
            self.total_cycle += 1.max(row_cycle);
        } else {
            self.total_cycle += row_cycle;
        }
        self.total_cycle += size.0;
    }
}
impl JumpCycle for IdealJumpCycle {
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

impl AddableJumpCycle for IdealJumpCycle {
    fn add(&mut self, ideal_jump_cycle: &IdealJumpCycle) {
        self.total_cycle += ideal_jump_cycle.total_cycle;
    }
}
