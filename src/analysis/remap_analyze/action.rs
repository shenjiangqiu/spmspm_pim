use super::row_cycle::*;

#[derive(Default)]
pub struct TotalAction {
    index: usize,
    pub total: [usize; TOTAL_TYPES_COUNT],
}
impl RowCycleAction for TotalAction {
    fn apply<T: JumpCycle + UpdatableJumpCycle + AddableJumpCycle>(&mut self, item: &T) {
        self.total[self.index] = item.total();
        self.index += 1;
    }
}

#[derive(Default)]
pub struct ReduceAction {
    pub total_cycles: [usize; TOTAL_TYPES_COUNT],
    index: usize,
}
impl RowCycleArrayReduce for ReduceAction {
    fn apply_reduce<T: JumpCycle + UpdatableJumpCycle + AddableJumpCycle>(
        &mut self,
        source: &[AllJumpCycles],
        target: &mut T,
        mapper: impl FnMut(&AllJumpCycles) -> &T,
    ) {
        let normal_jump_cycle = source.iter().map(mapper).max_by_key(|x| x.total()).unwrap();

        target.add(normal_jump_cycle);
        let total = normal_jump_cycle.total();
        self.total_cycles[self.index] = total;
    }
}

/// ## rust function
/// ## Author: Jiangqiu Shen
/// ## Date: 2023-05-11
/// Description: a Action which impl RowCycleActionMut, used to update the row cycle,
/// this action will call the update for all fields of the struct
pub struct UpdateAction<'a> {
    pub row_status: &'a RowIdWordId,
    pub loc: &'a RowLocation,
    pub size: WordId,
    pub remap_cycle: usize,
}
impl<'a> RowCycleActionMut for UpdateAction<'a> {
    fn apply_mut<T: JumpCycle + UpdatableJumpCycle + AddableJumpCycle>(&mut self, item: &mut T) {
        item.update(self.row_status, self.loc, self.size, self.remap_cycle);
    }
}

/// ## rust function
/// ## Author: Jiangqiu Shen
/// ## Date: 2023-05-24
/// Description: a Action which impl RowCycleActionMut, used to update the row cycle in batch mode
pub struct UpdateBatchAction<'a> {
    pub row_status: &'a RowIdWordId,
    pub loc: &'a [RowLocation],
    pub size: WordId,
    pub remap_cycle: usize,
}
impl<'a> RowCycleActionMut for UpdateBatchAction<'a> {
    fn apply_mut<T: JumpCycle + UpdatableJumpCycle + AddableJumpCycle>(&mut self, item: &mut T) {
        item.batch_update(self.row_status, self.loc, self.size, self.remap_cycle);
    }
}
