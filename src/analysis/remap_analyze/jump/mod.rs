// pub(crate) mod from_source;
mod histo;
pub(crate) mod ideal_jump;
pub(crate) mod myjump;
pub(crate) mod myjump_no_jump_overhead;
pub(crate) mod myjump_opt;
pub(crate) mod normal_jump;
// pub(crate) mod smart_jump;
use crate::analysis::{
    mapping::{PhysicRowId, WordId},
    translate_mapping::RowLocation,
};
// pub(crate) use from_source::FromSourceJumpCycle;
pub(crate) use ideal_jump::IdealJumpCycle;
pub(crate) use myjump::MyJumpCycle;
pub(crate) use myjump_no_jump_overhead::MyJumpNoOverhead;
pub(crate) use myjump_opt::MyJumpOpt;
pub(crate) use normal_jump::NormalJumpCycle;
// pub(crate) use smart_jump::SmartJumpCycle;
pub trait JumpCycle {
    fn total(&self) -> usize;
    fn get_one_jump(&self) -> usize;
    fn get_multi_jump(&self) -> usize;
    fn get_one_jump_mut(&mut self) -> &mut usize;
    fn get_multi_jump_mut(&mut self) -> &mut usize;
}
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
pub(crate) trait AddableJumpCycle: JumpCycle {
    fn add(&mut self, jump_cycle: &Self);
}

pub(crate) trait UpdatableJumpCycle {
    fn update(
        &mut self,
        row_status: &(PhysicRowId, WordId),
        loc: &RowLocation,
        size: WordId,
        remap_cycle: usize,
    );
}
