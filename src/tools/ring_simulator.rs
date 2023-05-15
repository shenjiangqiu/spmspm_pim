use std::collections::VecDeque;

use super::{Direction, IcntPacket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceId {
    Input,
    Left,
    Right,
}
impl SourceId {
    /// get the iter sequence of the source
    pub fn sequence(&self) -> [SourceId; 3] {
        match self {
            SourceId::Input => [SourceId::Input, SourceId::Left, SourceId::Right],
            SourceId::Left => [SourceId::Left, SourceId::Right, SourceId::Input],
            SourceId::Right => [SourceId::Right, SourceId::Input, SourceId::Left],
        }
    }
    /// set the source for next cycle
    pub fn next(&mut self) {
        *self = self.get_next();
    }
    /// get next source
    pub fn get_next(&self) -> Self {
        match self {
            SourceId::Input => SourceId::Left,
            SourceId::Left => SourceId::Right,
            SourceId::Right => SourceId::Input,
        }
    }
}

pub struct RingSimulator<T: IcntPacket> {
    /// the input buffer
    pub input: Vec<VecDeque<T>>,
    /// the output buffer
    pub output: Vec<VecDeque<T>>,
    /// the inner buffer
    left_buffer: Vec<VecDeque<T>>,
    right_buffer: Vec<VecDeque<T>>,
    /// the temp buffer
    temp_right_buffer: Vec<VecDeque<T>>,
    temp_left_buffer: Vec<VecDeque<T>>,
    creddit_left: Vec<usize>,
    creddit_right: Vec<usize>,
    source_id: SourceId,
    /// the total nodes of the ring
    nodes: usize,
    buffer_capacity: usize,
}
impl<T: IcntPacket> RingSimulator<T> {
    /// create a new ring simulator
    /// with `nodes` nodes
    pub fn new(nodes: usize, buffer_capacity: usize) -> Self {
        Self {
            input: (0..nodes)
                .map(|_| VecDeque::with_capacity(buffer_capacity))
                .collect(),
            output: (0..nodes)
                .map(|_| VecDeque::with_capacity(buffer_capacity))
                .collect(),
            left_buffer: (0..nodes)
                .map(|_| VecDeque::with_capacity(buffer_capacity))
                .collect(),
            right_buffer: (0..nodes)
                .map(|_| VecDeque::with_capacity(buffer_capacity))
                .collect(),
            temp_left_buffer: (0..nodes)
                .map(|_| VecDeque::with_capacity(buffer_capacity))
                .collect(),
            temp_right_buffer: (0..nodes)
                .map(|_| VecDeque::with_capacity(buffer_capacity))
                .collect(),
            source_id: SourceId::Input,
            nodes,
            creddit_left: vec![buffer_capacity; nodes],
            creddit_right: vec![buffer_capacity; nodes],
            buffer_capacity,
        }
    }
    /// add a value to the input buffer of a node
    pub fn add(&mut self, node: usize, value: T) -> Result<(), T> {
        if self.input[node].len() == self.buffer_capacity {
            Err(value)
        } else {
            // try to allocate the creddit from source .. target
            let mut source = value.get_source();
            let target = value.get_next_hop();
            let direction = value.get_direction();
            let creddit = match value.get_direction() {
                Direction::Left => &mut self.creddit_left,
                Direction::Right => &mut self.creddit_right,
            };
            while source != target {
                if creddit[source] == 0 {
                    // no creddit remain, return err
                    return Err(value);
                }
                match direction {
                    Direction::Left => {
                        source = (source + self.nodes - 1) % self.nodes;
                    }
                    Direction::Right => {
                        source = (source + 1) % self.nodes;
                    }
                }
            }
            // allocate the creddit
            let mut source = value.get_source();
            while source != target {
                creddit[source] -= 1;
                match direction {
                    Direction::Left => {
                        source = (source + self.nodes - 1) % self.nodes;
                    }
                    Direction::Right => {
                        source = (source + 1) % self.nodes;
                    }
                }
            }
            self.input[node].push_back(value);
            Ok(())
        }
    }

    /// pop a value from the output buffer of a node
    pub fn pop(&mut self, node: usize) -> Option<T> {
        // release the creddit
        let p = self.output[node].pop_front();
        match p {
            Some(p) => {
                //release the creddit
                let mut source = p.get_source();
                let target = p.get_next_hop();
                let direction = p.get_direction();
                let creddit = match p.get_direction() {
                    Direction::Left => &mut self.creddit_left,
                    Direction::Right => &mut self.creddit_right,
                };
                while source != target {
                    creddit[source] += 1;
                    match direction {
                        Direction::Left => {
                            source = (source + self.nodes - 1) % self.nodes;
                        }
                        Direction::Right => {
                            source = (source + 1) % self.nodes;
                        }
                    }
                }
                Some(p)
            }
            None => None,
        }
    }

    pub fn front(&self, node: usize) -> Option<&T> {
        self.output[node].front()
    }

    /// cycle the simulator
    pub fn cycle(&mut self) {
        // route packets
        // will push the packet to temp buffer first
        for node in 0..self.nodes {
            for direction in self.source_id.sequence() {
                match direction {
                    SourceId::Input => {
                        // an input packet can go to left, right or output
                        if let Some(packet) = self.input[node].pop_front() {
                            if packet.get_next_hop() == node {
                                if self.output[node].len() < self.buffer_capacity {
                                    // tracing::debug!("from input {} to output {}", node, node);
                                    self.output[node].push_back(packet);
                                } else {
                                    self.input[node].push_front(packet);
                                }
                            } else {
                                match packet.get_direction() {
                                    Direction::Left => {
                                        if self.left_buffer[node].len() < self.buffer_capacity {
                                            self.temp_left_buffer[node].push_back(packet);
                                            // tracing::debug!("from input {} to left {}", node, node);
                                        } else {
                                            self.input[node].push_front(packet);
                                        }
                                    }
                                    Direction::Right => {
                                        if self.right_buffer[node].len() < self.buffer_capacity {
                                            self.temp_right_buffer[node].push_back(packet);
                                            // tracing::debug!(
                                            //     "from input {} to right {}",
                                            //     node,
                                            //     node
                                            // );
                                        } else {
                                            self.input[node].push_front(packet);
                                        }
                                    }
                                }
                            }
                            // if the packet can't be routed, push it back to the input buffer
                        }
                    }
                    SourceId::Left => {
                        let right_node = (node + 1) % self.nodes;
                        if let Some(packet) = self.left_buffer[right_node].pop_front() {
                            if packet.get_next_hop() == node {
                                if self.output[node].len() < self.buffer_capacity {
                                    self.output[node].push_back(packet);
                                    // tracing::debug!("from left {} to output {}", right_node, node);
                                } else {
                                    self.left_buffer[right_node].push_front(packet);
                                }
                            } else if self.left_buffer[node].len() < self.buffer_capacity {
                                // tracing::debug!("from left {} to left {}", right_node, node);
                                self.temp_left_buffer[node].push_back(packet);
                            } else {
                                self.left_buffer[right_node].push_front(packet);
                            }
                            // if the packet can't be routed, push it back to the input buffer
                        }
                    }
                    SourceId::Right => {
                        let left_node = (node + self.nodes - 1) % self.nodes;
                        if let Some(packet) = self.right_buffer[left_node].pop_front() {
                            if packet.get_next_hop() == node {
                                if self.output[node].len() < self.buffer_capacity {
                                    self.output[node].push_back(packet);
                                    // tracing::debug!("from right {} to output {}", left_node, node);
                                } else {
                                    self.right_buffer[left_node].push_front(packet);
                                }
                            } else if self.right_buffer[node].len() < self.buffer_capacity {
                                self.temp_right_buffer[node].push_back(packet);
                                // tracing::debug!("from right {} to right {}", left_node, node);
                            } else {
                                self.right_buffer[left_node].push_front(packet);
                            }
                            // if the packet can't be routed, push it back to the input buffer
                        }
                    }
                }
            }
        }
        // finished the routing, move the source to the next
        self.source_id.next();
        // move the temp buffer to the real buffer
        for node in 0..self.nodes {
            self.left_buffer[node].append(&mut self.temp_left_buffer[node]);
            self.right_buffer[node].append(&mut self.temp_right_buffer[node]);
        }
    }
}

#[cfg(test)]
mod tests {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct TestPacket {
        next_hop: usize,
        direction: Direction,
        source: usize,
    }
    impl IcntPacket for TestPacket {
        fn get_next_hop(&self) -> usize {
            self.next_hop
        }
        fn get_direction(&self) -> Direction {
            self.direction
        }

        fn get_source(&self) -> usize {
            self.source
        }
    }

    use super::*;
    #[test]
    fn test_icnt_ring() {
        let mut simulator = RingSimulator::new(4, 16);
        simulator
            .add(
                0,
                TestPacket {
                    next_hop: 0,
                    direction: Direction::Left,
                    source: 0,
                },
            )
            .unwrap();
        simulator.cycle();
        let out = simulator.pop(0).unwrap();
        assert_eq!(
            out,
            TestPacket {
                next_hop: 0,
                direction: Direction::Left,
                source: 0,
            }
        );
    }

    #[test]
    fn test_icnt_left() {
        let mut simulator = RingSimulator::new(4, 16);
        let packet = TestPacket {
            next_hop: 3,
            direction: Direction::Left,
            source: 0,
        };
        simulator.add(0, packet).unwrap();
        simulator.cycle();
        assert_eq!(simulator.left_buffer[0].front(), Some(&packet));
        simulator.cycle();
        let out = simulator.pop(3).unwrap();
        assert_eq!(out, packet);
    }

    #[test]
    fn test_jump_two() {
        let mut simulator = RingSimulator::new(4, 16);
        let packet = TestPacket {
            next_hop: 2,
            direction: Direction::Left,
            source: 0,
        };
        simulator.add(0, packet).unwrap();
        simulator.cycle();
        assert_eq!(simulator.left_buffer[0].front(), Some(&packet));
        simulator.cycle();
        assert_eq!(simulator.left_buffer[3].front(), Some(&packet));
        simulator.cycle();
        let out = simulator.pop(2).unwrap();
        assert_eq!(out, packet);
    }

    #[test]
    fn test_jump_right() {
        let mut simulator = RingSimulator::new(4, 16);
        let packet = TestPacket {
            next_hop: 1,
            direction: Direction::Right,
            source: 0,
        };
        simulator.add(0, packet).unwrap();
        simulator.cycle();
        assert_eq!(simulator.right_buffer[0].front(), Some(&packet));
        simulator.cycle();
        let out = simulator.pop(1).unwrap();
        assert_eq!(out, packet);
    }
    #[test]
    fn test_jump_right_two() {
        let mut simulator = RingSimulator::new(4, 16);
        let packet = TestPacket {
            next_hop: 2,
            direction: Direction::Right,
            source: 0,
        };
        simulator.add(0, packet).unwrap();
        simulator.cycle();
        assert_eq!(simulator.right_buffer[0].front(), Some(&packet));
        simulator.cycle();
        assert_eq!(simulator.right_buffer[1].front(), Some(&packet));
        simulator.cycle();
        let out = simulator.pop(2).unwrap();
        assert_eq!(out, packet);
    }

    #[test]
    fn test_conflict() {
        let mut simulator = RingSimulator::new(4, 16);
        let p1 = TestPacket {
            next_hop: 1,
            direction: Direction::Right,
            source: 0,
        };
        let p2 = TestPacket {
            next_hop: 1,
            direction: Direction::Left,
            source: 2,
        };
        let p3 = TestPacket {
            next_hop: 1,
            direction: Direction::Left,
            source: 1,
        };
        simulator.add(0, p1).unwrap();
        simulator.add(2, p2).unwrap();
        simulator.cycle();
        assert_eq!(simulator.right_buffer[0].front(), Some(&p1));
        assert_eq!(simulator.left_buffer[2].front(), Some(&p2));
        simulator.add(1, p3).unwrap();
        simulator.cycle();
        assert_eq!(simulator.right_buffer[0].front(), None);
        assert_eq!(simulator.left_buffer[2].front(), None);
        assert_eq!(simulator.input[1].front(), None);

        assert_eq!(simulator.output[1].len(), 3);
    }

    #[test]
    fn test_output_full() {
        let mut simulator = RingSimulator::new(4, 4);
        let p1 = TestPacket {
            next_hop: 1,
            direction: Direction::Right,
            source: 0,
        };
        for _i in 0..4 {
            simulator.add(0, p1).unwrap();
            // tracing::debug!(i);
            simulator.cycle();
        }
        simulator.cycle();
        // assert_eq!(simulator.right_buffer[0].len(), 4);
        assert_eq!(simulator.output[1].len(), 4);
        // no credit
        let result = simulator.add(0, p1);
        assert_eq!(result, Err(p1));
    }

    #[test]
    fn test_output_full_left() {
        let mut simulator = RingSimulator::new(4, 4);
        let p1 = TestPacket {
            next_hop: 3,
            direction: Direction::Left,
            source: 0,
        };
        for _ in 0..4 {
            simulator.add(0, p1).unwrap();
            simulator.cycle();
        }
        simulator.cycle();
        // assert_eq!(simulator.right_buffer[0].len(), 4);
        assert_eq!(simulator.output[3].len(), 4);
        let result = simulator.add(0, p1);
        assert_eq!(result, Err(p1));
    }

    #[test]
    fn test_full_bandwith_long_ring() {
        run_full_bandwidth(Direction::Right, 16, |i| (i + 16 - 1) % 16);
        run_full_bandwidth(Direction::Left, 16, |i| (i + 1) % 16);
        run_full_bandwidth(Direction::Right, 16, |i| (i + 1) % 16);
        run_full_bandwidth(Direction::Right, 16, |i| (i + 2) % 16);
    }

    fn run_full_bandwidth(
        direction: Direction,
        num_ports: usize,
        target_fn: impl Fn(usize) -> usize,
    ) {
        let mut simulator = RingSimulator::new(num_ports, num_ports * num_ports);
        let mut packs: Vec<_> = (0..num_ports)
            .map(|i| {
                let target_port = target_fn(i);
                (0..1000)
                    .map(move |_| TestPacket {
                        next_hop: target_port,
                        direction,
                        source: i,
                    })
                    .peekable()
            })
            .collect();

        let mut total_finished = 0;
        let mut total_cycle = 0;
        while total_finished != num_ports * 1000 {
            // push packets into the simulator
            for (i, p) in packs.iter_mut().enumerate() {
                if let Some(packet) = p.peek() {
                    if simulator.add(i, *packet).is_ok() {
                        p.next().unwrap();
                    };
                }
            }
            simulator.cycle();
            for i in 0..num_ports {
                if simulator.pop(i).is_some() {
                    total_finished += 1;
                    // println!(
                    //     "total finished: {}/{} at port {}",
                    //     total_finished,
                    //     num_ports * 1000,
                    //     i
                    // );
                }
            }
            total_cycle += 1;
        }
        println!("total cycle: {}", total_cycle);
    }
}
