// pub(crate) mod from_source;
// mod histo;
mod ideal_jump;
mod myjump;
mod myjump_no_jump_overhead;
mod myjump_only;
mod myjump_opt;
mod normal_jump;

// pub(crate) mod smart_jump;

// pub(crate) use from_source::FromSourceJumpCycle;
use super::row_cycle::{
    AddableJumpCycle, JumpCycle, RowIdWordId, RowLocation, UpdatableJumpCycle, WordId,
};
pub use ideal_jump::IdealJumpCycle;
pub use myjump::MyJumpCycle;
pub use myjump_no_jump_overhead::MyJumpNoOverhead;
pub use myjump_only::MyJumpOnly;
pub use myjump_opt::MyJumpOpt;
pub use normal_jump::NormalJumpCycle;

pub(crate) trait AddTwo {
    fn add_two(&mut self);
}
pub(crate) struct TargetSourcePair<'a, T> {
    pub(crate) target: &'a mut T,
    pub(crate) source: &'a T,
}

impl<'a, T> AddTwo for (&'a mut T, &'a T)
where
    T: AddableJumpCycle,
{
    fn add_two(&mut self) {
        self.0.add(self.1);
    }
}

impl<'a, T> AddTwo for TargetSourcePair<'a, T>
where
    T: AddableJumpCycle,
{
    fn add_two(&mut self) {
        self.target.add(self.source);
    }
}

pub(crate) fn check_same_walker<const WALKER_SIZE: usize>(
    source: &RowIdWordId,
    target: &RowIdWordId,
) -> bool {
    source.row_id == target.row_id
        && (source.word_id.0 * 4 / WALKER_SIZE == target.word_id.0 * 4 / WALKER_SIZE)
}
/// ## rust function
/// ## Author: Jiangqiu Shen
/// ## Date: 2023-05-15
/// Description: return the frist row miss times and remaining misses
pub fn get_total_row_cycle<const WALKER_SIZE: usize>(
    row_status: &RowIdWordId,
    loc: &RowLocation,
    size: WordId,
) -> (usize, usize) {
    let first_row_miss = if check_same_walker::<WALKER_SIZE>(row_status, &loc.row_id_world_id) {
        0
    } else {
        1
    };
    let walkers_to_load =
        get_num_extra_walkers_to_load::<WALKER_SIZE>(loc.row_id_world_id.word_id, size);

    (first_row_miss, walkers_to_load)
}
pub fn get_num_extra_walkers_to_load<const WALKER_SIZE: usize>(
    start_world_id: WordId,
    size: WordId,
) -> usize {
    if size.0 == 0 || size.0 == 1 {
        // always load the same walker as start_world_id
        return 0;
    }
    let end_world_id = start_world_id.0 + size.0 - 1;
    let start_partiton = start_world_id.0 / (WALKER_SIZE / 4);
    let end_partiton = end_world_id / (WALKER_SIZE / 4);

    end_partiton - start_partiton
}

#[cfg(test)]
mod tests {

    use crate::analysis::remap_analyze::row_cycle::{PhysicRowId, WordId};

    use super::*;
    #[test]
    fn test_check_same_walker() {
        let source = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(0),
        };
        let target = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(3),
        };
        let false_target_wrong_row = RowIdWordId {
            row_id: PhysicRowId(1),
            word_id: WordId(3),
        };
        let false_target_wrong_col = RowIdWordId {
            row_id: PhysicRowId(0),
            word_id: WordId(8),
        };
        assert!(check_same_walker::<32>(&source, &target));
        assert!(!check_same_walker::<32>(&source, &false_target_wrong_row));
        assert!(!check_same_walker::<32>(&source, &false_target_wrong_col));
    }
}
