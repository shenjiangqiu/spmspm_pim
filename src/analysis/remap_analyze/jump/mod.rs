// pub(crate) mod from_source;
// mod histo;
pub(crate) mod ideal_jump;
pub(crate) mod myjump;
pub(crate) mod myjump_no_jump_overhead;
pub(crate) mod myjump_only;
pub(crate) mod myjump_opt;
pub(crate) mod normal_jump;
use crate::analysis::remap_analyze::row_cycle::*;

// pub(crate) mod smart_jump;

// pub(crate) use from_source::FromSourceJumpCycle;
use super::row_cycle::{AddableJumpCycle, JumpCycle, UpdatableJumpCycle};
pub(crate) use ideal_jump::IdealJumpCycle;
pub(crate) use myjump::MyJumpCycle;
pub(crate) use myjump_no_jump_overhead::MyJumpNoOverhead;
pub(crate) use myjump_only::MyJumpOnly;
pub(crate) use myjump_opt::MyJumpOpt;
pub(crate) use normal_jump::NormalJumpCycle;
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

#[cfg(test)]
mod tests {

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
