mod flat_interleave;
pub use flat_interleave::{FlatInterleave, FlatInterleaveTrait};
pub mod crossbare_simulator;
pub mod crossbare_simulator_no_conflic;
pub mod math;
pub mod ring_simulator;
pub trait CrossBarPacket {
    fn get_source(&self) -> usize;
    fn get_dest(&self) -> usize;
}
pub trait IcntPacket {
    fn get_source(&self) -> usize;
    fn get_next_hop(&self) -> usize;
    fn get_direction(&self) -> Direction;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}
