pub mod from_source;
pub mod ideal_jump;
pub mod myjump;
pub mod normal_jump;
pub mod smart_jump;

pub use from_source::FromSourceJumpCycle;
pub use ideal_jump::IdealJumpCycle;
pub use myjump::MyJumpCycle;
pub use normal_jump::NormalJumpCycle;
pub use smart_jump::SmartJumpCycle;

pub trait JumpCycle {
    fn total(&self) -> usize;
    fn add(&mut self, other: &Self);
}
