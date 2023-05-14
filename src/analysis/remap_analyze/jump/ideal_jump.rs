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
        let row_cycle = get_total_row_cycle::<WALKER_SIZE>(row_status, loc, size);

        if loc.row_id_world_id.word_id != row_status.word_id {
            // it' not the same col
            self.total_cycle += 1.max(row_cycle);
        } else {
            self.total_cycle += row_cycle;
        }
        self.total_cycle += size.0;
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
