pub(crate) mod from_source;
pub(crate) mod ideal_jump;
pub(crate) mod myjump;
pub(crate) mod myjump_no_jump_overhead;
pub(crate) mod myjump_opt;
pub(crate) mod normal_jump;
pub(crate) mod smart_jump;
pub(crate) use from_source::FromSourceJumpCycle;
pub(crate) use ideal_jump::IdealJumpCycle;
pub(crate) use myjump::MyJumpCycle;
pub(crate) use myjump_no_jump_overhead::MyJumpNoOverhead;
pub(crate) use myjump_opt::MyJumpOpt;
pub(crate) use normal_jump::NormalJumpCycle;
pub(crate) use smart_jump::SmartJumpCycle;

use crate::analysis::translate_mapping::RowLocation;
pub trait JumpCycle {
    fn total(&self) -> usize;
    fn get_one_jump(&self) -> usize;
    fn get_multi_jump(&self) -> usize;
    fn get_one_jump_mut(&mut self) -> &mut usize;
    fn get_multi_jump_mut(&mut self) -> &mut usize;
}
pub(crate) trait AddableJumpCycle: JumpCycle {
    fn add(&mut self, jump_cycle: &Self);
}

pub(crate) trait UpdatableJumpCycle {
    fn update(
        &mut self,
        row_status: &(usize, usize),
        loc: &RowLocation,
        size: usize,
        remap_cycle: usize,
    );
}
