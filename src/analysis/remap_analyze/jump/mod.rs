pub mod from_source;
pub mod ideal_jump;
pub mod myjump;
pub mod myjump_no_jump_overhead;
pub mod myjump_opt;
pub mod normal_jump;
pub mod smart_jump;
pub use from_source::FromSourceJumpCycle;
pub use ideal_jump::IdealJumpCycle;
pub use myjump::MyJumpCycle;
pub use myjump_no_jump_overhead::MyJumpNoOverhead;
pub use myjump_opt::MyJumpOpt;
pub use normal_jump::NormalJumpCycle;
pub use smart_jump::SmartJumpCycle;
pub trait JumpCycle {
    fn total(&self) -> usize;
    fn get_one_jump(&self) -> usize;
    fn get_multi_jump(&self) -> usize;
    fn get_one_jump_mut(&mut self) -> &mut usize;
    fn get_multi_jump_mut(&mut self) -> &mut usize;
}
pub trait AddableJumpCycle: JumpCycle {
    fn add(&mut self, jump_cycle: &Self);
}
