use std::{borrow::Borrow, collections::BTreeMap};

use itertools::Itertools;

use crate::analysis::remap_analyze::row_cycle::*;

pub struct SelectiveUpdator<'a, const WALKER_SIZE: usize, T: UpdatableJumpCycle> {
    pub extra_scan_cycles: usize,
    pub jump_cycle: &'a mut T,
    pub row_status: RowIdWordId,
    pub size: WordId,
    pub remap_cycle: usize,
}
impl<'a, const WALKER_SIZE: usize, T: UpdatableJumpCycle> SelectiveUpdator<'a, WALKER_SIZE, T> {
    pub fn new(
        jump_cycle: &'a mut T,
        row_status: RowIdWordId,
        size: WordId,
        remap_cycle: usize,
    ) -> Self {
        Self {
            extra_scan_cycles: 0,
            jump_cycle,
            row_status,
            size,
            remap_cycle,
        }
    }
}

impl<const WALKER_SIZE: usize, T: UpdatableJumpCycle> super::RemoteUpdator
    for SelectiveUpdator<'_, WALKER_SIZE, T>
{
    fn update<Item: Borrow<RowLocation>>(&mut self, data: impl IntoIterator<Item = Item>) {
        // first we need to split the tasks into different walkers, each data is (col,data) is 8 Bytes, so there are WALKER_SIZE/8 tasks in one walker
        for task in data.into_iter().chunks(WALKER_SIZE / 8).into_iter() {
            // group the tasks by row_id and walker_id
            // we need btree map here because the key sequence is important!! (from small to large)
            let tasks: BTreeMap<_, _> = task
                .into_group_map_by(|x| {
                    (
                        x.borrow().row_id_word_id.row_id,
                        x.borrow().row_id_word_id.word_id.0 * 4 / WALKER_SIZE,
                    )
                })
                .into_iter()
                .collect();

            if tasks.is_empty() {
                continue;
            }
            self.extra_scan_cycles += (tasks.len() - 1) * WALKER_SIZE / 4;

            // for each round, update the result
            for ((_row_id, _walker_id), locs) in tasks {
                for loc in locs {
                    self.jump_cycle.update(
                        &self.row_status,
                        loc.borrow(),
                        self.size,
                        self.remap_cycle,
                    );
                    self.row_status = loc.borrow().row_id_word_id;
                }
            }
        }
    }
}
