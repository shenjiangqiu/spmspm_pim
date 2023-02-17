use std::collections::{BTreeSet, VecDeque};

use super::CrossBarPacket;

pub struct CrossBareSimulator<T> {
    ports: usize,
    buffer_size: usize,
    input_buffer: Vec<VecDeque<T>>,
    output_buffer: Vec<VecDeque<T>>,
    shift: usize,
    pub target_conflict: usize,
}

impl<T> CrossBareSimulator<T> {
    pub fn new(ports: usize, buffer_size: usize) -> Self {
        Self {
            ports,
            buffer_size,
            input_buffer: (0..ports)
                .map(|_| VecDeque::with_capacity(buffer_size))
                .collect(),
            output_buffer: (0..ports)
                .map(|_| VecDeque::with_capacity(buffer_size))
                .collect(),
            shift: 0,
            target_conflict: 0,
        }
    }
    pub fn add(&mut self, port: usize, packet: T) -> Result<(), T> {
        if self.input_buffer[port].len() < self.buffer_size {
            self.input_buffer[port].push_back(packet);
            Ok(())
        } else {
            Err(packet)
        }
    }
    pub fn pop(&mut self, port: usize) -> Option<T> {
        self.output_buffer[port].pop_front()
    }
}

impl<T: CrossBarPacket> CrossBareSimulator<T> {
    pub fn cycle(&mut self) {
        let mut current_round_target = BTreeSet::new();
        for i in 0..self.ports {
            let index = (i + self.shift) % self.ports;
            if let Some(traffic) = self.input_buffer[index].pop_front() {
                // already in current round
                let target = traffic.get_dest();
                if current_round_target.contains(&traffic.get_dest()) {
                    self.input_buffer[index].push_front(traffic);
                    self.target_conflict += 1;
                } else if self.output_buffer[target].len() >= self.buffer_size {
                    self.input_buffer[index].push_front(traffic);
                } else {
                    current_round_target.insert(traffic.get_dest());
                    self.output_buffer[traffic.get_dest()].push_back(traffic);
                }
            }
        }
        self.shift = (self.shift + 1) % self.ports;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    struct TestPacket {
        source: usize,
        dest: usize,
    }
    impl CrossBarPacket for TestPacket {
        fn get_source(&self) -> usize {
            self.source
        }
        fn get_dest(&self) -> usize {
            self.dest
        }
    }
    #[test]
    fn test1() {
        let mut sim = CrossBareSimulator::new(4, 2);
        sim.add(0, TestPacket { source: 0, dest: 1 }).unwrap();
        sim.add(0, TestPacket { source: 0, dest: 3 }).unwrap();
        assert_eq!(
            sim.add(0, TestPacket { source: 0, dest: 3 }),
            Err(TestPacket { source: 0, dest: 3 })
        );
        sim.add(1, TestPacket { source: 1, dest: 2 }).unwrap();
        sim.add(1, TestPacket { source: 1, dest: 0 }).unwrap();
        sim.add(2, TestPacket { source: 2, dest: 3 }).unwrap();
        sim.add(2, TestPacket { source: 2, dest: 1 }).unwrap();
        sim.add(3, TestPacket { source: 3, dest: 0 }).unwrap();
        sim.add(3, TestPacket { source: 3, dest: 2 }).unwrap();
        sim.cycle();
        assert_eq!(sim.pop(0), Some(TestPacket { source: 3, dest: 0 }));
        assert_eq!(sim.pop(1), Some(TestPacket { source: 0, dest: 1 }));
        assert_eq!(sim.pop(2), Some(TestPacket { source: 1, dest: 2 }));
        assert_eq!(sim.pop(3), Some(TestPacket { source: 2, dest: 3 }));
        sim.cycle();
        assert_eq!(sim.pop(0), Some(TestPacket { source: 1, dest: 0 }));
        assert_eq!(sim.pop(1), Some(TestPacket { source: 2, dest: 1 }));
        assert_eq!(sim.pop(2), Some(TestPacket { source: 3, dest: 2 }));
        assert_eq!(sim.pop(3), Some(TestPacket { source: 0, dest: 3 }));
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), None);
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);

        assert_eq!(sim.input_buffer[0].len(), 0);
        assert_eq!(sim.input_buffer[1].len(), 0);
        assert_eq!(sim.input_buffer[2].len(), 0);
        assert_eq!(sim.input_buffer[3].len(), 0);
    }

    #[test]
    fn test_conflict() {
        let mut sim = CrossBareSimulator::new(4, 2);
        sim.add(0, TestPacket { source: 0, dest: 1 }).unwrap();
        sim.add(0, TestPacket { source: 0, dest: 1 }).unwrap();
        sim.add(1, TestPacket { source: 1, dest: 1 }).unwrap();
        sim.add(1, TestPacket { source: 1, dest: 1 }).unwrap();
        sim.add(2, TestPacket { source: 2, dest: 1 }).unwrap();
        sim.add(2, TestPacket { source: 2, dest: 1 }).unwrap();
        sim.add(3, TestPacket { source: 3, dest: 1 }).unwrap();
        sim.add(3, TestPacket { source: 3, dest: 1 }).unwrap();
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 0, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 1, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 2, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 3, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 0, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 1, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 2, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);
        sim.cycle();
        assert_eq!(sim.pop(0), None);
        assert_eq!(sim.pop(1), Some(TestPacket { source: 3, dest: 1 }));
        assert_eq!(sim.pop(2), None);
        assert_eq!(sim.pop(3), None);

        assert_eq!(sim.input_buffer[0].len(), 0);
        assert_eq!(sim.input_buffer[1].len(), 0);
        assert_eq!(sim.input_buffer[2].len(), 0);
        assert_eq!(sim.input_buffer[3].len(), 0);
    }
}
