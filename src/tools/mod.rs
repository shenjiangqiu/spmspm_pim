pub mod file_server;
mod flat_interleave;
mod rayon_play;
pub use flat_interleave::{FlatInterleave, FlatInterleaveTrait};
pub mod crossbare_simulator;
pub mod crossbare_simulator_no_conflic;
pub mod math;
pub mod remapping_translate;
pub mod ring_simulator;
pub mod stop_signal;
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

#[cfg(test)]
mod tests {

    use itertools::Itertools;

    #[test]
    fn iter_tools_test() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        let b: Vec<_> = a
            .into_iter()
            .group_by(|x| *x % 4 == 0)
            .into_iter()
            .map(|(k, v)| (k, v.collect_vec()))
            .collect();
        println!("{:?}", b);
    }

    #[test]
    fn test_grouping() {
        let a = [(1, 2), (3, 4), (5, 6), (7, 8), (1, 3)];
        let b = a.into_iter().into_group_map();
        println!("{:?}", b);
        let a = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let b = a.into_iter().into_group_map_by(|k| k % 2);
        println!("{:?}", b);
    }

    #[test]
    fn test_grouping2() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let b = a
            .into_iter()
            .into_grouping_map_by(|k| k % 2)
            .fold_first(|r, _k, v| r + v);
        println!("{:?}", b);
    }
}
