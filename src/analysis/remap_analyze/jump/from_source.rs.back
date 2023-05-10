use serde::{Deserialize, Serialize};

use crate::analysis::{
    mapping::{PhysicRowId, WordId},
    translate_mapping::RowLocation,
};

use super::{AddableJumpCycle, JumpCycle, UpdatableJumpCycle};

#[derive(Default, Clone, Serialize, Deserialize, Debug, Copy)]
pub struct FromSourceJumpCycle {
    pub jump_one_cycle: usize,
    pub jump_multiple_cycle: usize,
}
impl UpdatableJumpCycle for FromSourceJumpCycle {
    fn update(
        &mut self,
        _row_status: &(PhysicRowId, WordId),
        location: &RowLocation,
        size: WordId,
        _remap_cycle: usize,
    ) {
        let row_cycle = if location.row_id == _row_status.0 {
            0
        } else {
            18
        };

        if location.col_id.0 > 4 {
            self.jump_multiple_cycle += (location.col_id.0 + 1).max(row_cycle);
        } else {
            self.jump_one_cycle += (location.col_id.0 + 1).max(row_cycle);
        }
        self.jump_one_cycle += size * 4;
    }
}
impl JumpCycle for FromSourceJumpCycle {
    fn total(&self) -> usize {
        self.jump_one_cycle + self.jump_multiple_cycle
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

impl AddableJumpCycle for FromSourceJumpCycle {
    fn add(&mut self, from_source_jump_cycle: &FromSourceJumpCycle) {
        self.jump_one_cycle += from_source_jump_cycle.jump_one_cycle;
        self.jump_multiple_cycle += from_source_jump_cycle.jump_multiple_cycle;
    }
}
