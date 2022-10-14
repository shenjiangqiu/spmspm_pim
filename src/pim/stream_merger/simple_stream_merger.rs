use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Debug,
};

use crate::pim::{
    config::Config,
    level::LevelTrait,
    task::{self, StreamMessage, StreamMessageData, Task, TaskType},
    Component, SimulationContext,
};

use super::{StreamProvider, TaskReceiver};

#[derive(Debug, Clone)]
pub enum Waiting {
    Idle,
    /// the merger is waiting for a task
    WaitingForTask,
    /// the merger is waiting for data
    WaitingForData,
    /// waiting for draining
    WaitingForDrain,
}

/// the status for a single merger
/// - idle: no task is being processed
/// - WaitingTasks: currently receiving tasks and recording the status
/// - WatitingData: finished receiving tasks, now waiting for data
#[derive(Debug, Clone)]
pub struct MergerStatus {
    status: Waiting,
    max_buffered_message: usize,
    max_generated_message: usize,
    max_stored_generated_message: usize,
    /// the data that finished processing
    generated_message: VecDeque<StreamMessage>,
    pub current_target: usize,
    /// when receiving a task, will increase the counter
    /// it means how many data needed to be received from each child
    pub waiting_data_childs: BTreeSet<usize>,
    /// the message stored from lower level
    pub buffered_message: Vec<VecDeque<StreamMessageData>>,
}

impl MergerStatus {
    /// return if finished
    /// - will try to merger the input,
    /// - will return true is all done
    fn cycle<LevelType>(
        &mut self,
        context: &mut SimulationContext<LevelType>,
        current_cycle: u64,
    ) -> bool {
        match self.status {
            Waiting::WaitingForData => {
                for _ in 0..self.max_generated_message {
                    // the result buffer is full
                    if self.generated_message.len() >= self.max_stored_generated_message {
                        return false;
                    }
                    // the input is not enough
                    for waiting_child in self.waiting_data_childs.iter() {
                        if self.buffered_message[*waiting_child].is_empty() {
                            return false;
                        }
                    }

                    // generate one result
                    match self.generate_one_msg() {
                        Some(idx) => {
                            let msg = context.message_builder.generate_msg(
                                self.current_target,
                                idx,
                                current_cycle,
                            );
                            self.generated_message.push_back(msg);
                        }
                        // no message generated, means that all input is empty
                        None => {
                            // generate a end message if all input is empty and switch to WaitingForDrain
                            let end_message = context.generate_end(self.current_target);
                            self.generated_message.push_back(end_message);
                            self.status = Waiting::WaitingForDrain;
                        }
                    }
                }
                return false;
                // all data received
            }
            Waiting::WaitingForDrain => {
                //  decide if all data is drained
                if self.generated_message.is_empty() {
                    self.status = Waiting::Idle;
                    return true;
                } else {
                    return false;
                }
            }
            _ => {
                return false;
            }
        }
    }

    fn generate_one_msg(&mut self) -> Option<usize> {
        if let Some(min_idx) = self
            .buffered_message
            .iter()
            .filter(|v| !v.is_empty())
            .map(|q| q.front().unwrap().idx)
            .min()
        {
            self.buffered_message.iter_mut().for_each(|v| {
                if let Some(msg) = v.front() {
                    if msg.idx == min_idx {
                        v.pop_front();
                    }
                }
            });

            return Some(min_idx);
        } else {
            return None;
        }
    }
}

impl MergerStatus {
    pub fn new(
        num_child: usize,
        max_buffered_message: usize,
        max_generated_message: usize,
        max_stored_generated_message: usize,
    ) -> Self {
        Self {
            status: Waiting::Idle,
            max_buffered_message,
            current_target: 0,
            waiting_data_childs: BTreeSet::new(),
            buffered_message: vec![VecDeque::with_capacity(max_buffered_message); num_child],
            max_generated_message,
            max_stored_generated_message,
            generated_message: VecDeque::with_capacity(max_stored_generated_message),
        }
    }
}

impl MergerStatus {
    pub fn receive_task(&mut self, to: usize, child_id: usize, num_children: usize) {
        match self.status {
            Waiting::Idle => {
                self.status = Waiting::WaitingForData;
                self.current_target = to;
                self.waiting_data_childs.insert(child_id);
            }
            Waiting::WaitingForTask => {
                if self.current_target != to {
                    panic!("received task to different target");
                }
                self.waiting_data_childs.insert(child_id);
            }
            _ => unreachable!(),
        }
    }
    pub fn receive_end(&mut self) {
        match self.status {
            Waiting::Idle => {
                // should be a stat signal
                unreachable!()
            }
            Waiting::WaitingForTask => self.status = Waiting::WaitingForData,
            _ => unreachable!(),
        }
    }
}

/// a simple stream merger
/// - it represent a merger pe in a certain level(like a channel)
/// - it will contains several mergers, each merger will be responsible for a certain target
/// - it will have several children, the child will present a lower merger or a lower provider
#[derive(Debug, Clone)]
pub struct SimpleStreamMerger<LevelType, Child> {
    /// the level of this merger
    current_level: LevelType,
    /// the children of this merger
    children: Vec<Child>,
    /// the status of each merger
    mergers: Vec<MergerStatus>,
    /// current receiving targets, target_id to pe_id
    current_receiving_targets: BTreeMap<usize, usize>,
    /// current working targets, target_id to pe_id
    current_working_targets: BTreeMap<usize, usize>,
    /// free pes
    free_pes: BTreeSet<usize>,

    /// max buffered data
    max_buffered_data: usize,
    /// max data provided in a cycle
    max_data_provided: usize,
}

impl<LevelType: LevelTrait, Child> SimpleStreamMerger<LevelType, Child> {
    /// create a new simple stream merger
    pub fn new(
        config: &Config,
        children: Vec<Child>,
        level: LevelType,
        num_merger: usize,
        max_buffered_data: usize,
        max_data_provided: usize,
        max_buffered_message: usize,
        max_generated_message: usize,
        max_stored_generated_message: usize,
    ) -> Self {
        let num_child = children.len();
        Self {
            current_level: level,
            children,
            mergers: vec![
                MergerStatus::new(
                    num_child,
                    max_buffered_message,
                    max_generated_message,
                    max_stored_generated_message
                );
                num_merger
            ],
            current_receiving_targets: BTreeMap::new(),
            current_working_targets: BTreeMap::new(),
            free_pes: (0..num_merger).collect(),
            max_buffered_data,
            max_data_provided,
        }
    }
    fn can_receive_data(
        merger_status: &[MergerStatus],
        child_id: usize,
        data: &[&StreamMessage],
        current_working_map: &BTreeMap<usize, usize>,
    ) -> bool {
        for i in data {
            let to = i.to;
            let pe_id = current_working_map[&to];
            let merger = &merger_status[pe_id];
            if merger.buffered_message[child_id].len() >= merger.max_buffered_message {
                return false;
            }
        }
        true
    }
    fn receive_data(
        merger_status: &mut [MergerStatus],
        data: impl IntoIterator<Item = StreamMessage>,
        child_id: usize,
        current_working_map: &mut BTreeMap<usize, usize>,
    ) {
        for message in data {
            let to = message.to;
            let pe_id = current_working_map[&to];
            let merger = &mut merger_status[pe_id];
            match message.message_type {
                task::StreamMessageType::Data(data) => {
                    merger.buffered_message[child_id].push_back(data);
                }
                task::StreamMessageType::End => {
                    merger.waiting_data_childs.remove(&child_id);
                }
            }
        }
    }

    /// should not be used by user,
    /// use `receive_task` instead
    fn can_self_receive_task(&self, target_id: usize) -> bool {
        if self.current_receiving_targets.contains_key(&target_id) {
            return true;
        }
        return self.current_receiving_targets.len() < self.mergers.len();
    }
    /// should not be used by user,
    /// use `receive_task` instead
    /// ### **only use this after `can_self_receive_task` returns true**
    fn self_receive_task(
        &mut self,
        task_data: &task::TaskData<LevelType::Storage>,
        child_id: usize,
    ) {
        // yes we can receive this task

        let to = task_data.to;
        let entry = self.current_receiving_targets.entry(to).or_insert_with(|| {
            let pe_id = self.free_pes.iter().next().unwrap().clone();
            self.free_pes.remove(&pe_id);
            pe_id
        });
        let pe = &mut self.mergers[*entry];
        pe.receive_task(to, child_id, self.children.len());
    }

    /// receive a end task
    fn self_receive_end(&mut self, to: usize) {
        if let Some(pe_id) = self.current_receiving_targets.remove(&to) {
            let pe = &mut self.mergers[pe_id];
            pe.receive_end();
            self.current_working_targets.insert(to, pe_id);
        }
    }
}

impl<LevelType, Child> TaskReceiver for SimpleStreamMerger<LevelType, Child>
where
    Child: TaskReceiver<
        InputTask = Task<LevelType::Storage>,
        SimContext = SimulationContext<LevelType>,
        LevelType = LevelType,
    >,
    LevelType: LevelTrait + Debug,
{
    type InputTask = Task<LevelType::Storage>;
    type SimContext = SimulationContext<LevelType>;
    type LevelType = LevelType;
    /// it can receive the task only when it's ready and it's child is ready
    /// - if self is not ready, return error
    /// - if any of it's child is not ready, return error
    /// - if all of it's child is ready, perform change, return Ok
    /// ## conclusion
    /// - if it return ok, means it and all of it's child is ready, and it and all it's children have already changed it's status
    /// - if it return error, means it or any of it's child is not ready, and it nor all it's children have not changed it's status
    /// ## recursive invariant proof
    /// - when it's not ready, it will return error, correct!
    /// - when it's child return error, return error and no change happened, correct!
    /// - when it's child return ok, it will change it's status, and all it's children have already changed it's status, correct!
    fn receive_task(
        &mut self,
        task: &Self::InputTask,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Result<(), Self::LevelType> {
        // recursively receive task
        //

        // check if self can accept the task
        match task {
            Task::TaskData(task_data) => {
                if !self.can_self_receive_task(task_data.to) {
                    return Err(self.current_level.clone());
                }
            }
            Task::End(_end_task) => {
                // yes we can
            }
        }
        // send task to lower pe and record the task
        let child_level = self
            .current_level
            .get_child_level()
            .expect("no child level");

        match task {
            Task::TaskData(task_data) => {
                let child_id = task_data.target_id.get_level_id(&child_level);
                // first test if self can receive this task
                self.children[child_id].receive_task(task, context, current_cycle)?;
                // record this task
                self.self_receive_task(task_data, child_id);
                Ok(())
            }
            Task::End(end_data) => {
                // broadcast to all child
                for child in self.children.iter_mut() {
                    child.receive_task(task, context, current_cycle).unwrap();
                }
                self.self_receive_end(end_data.to);
                Ok(())
            }
        }
    }
}

impl<LevelType, Child> StreamProvider for SimpleStreamMerger<LevelType, Child> {
    type OutputData = StreamMessage;

    type SimContext = SimulationContext<LevelType>;
    /// return at most `max_data_provided` data
    fn get_data(
        &mut self,
        _context: &mut Self::SimContext,
        _current_cycle: u64,
    ) -> Vec<Self::OutputData> {
        // check self.data, pop at most `max_data_provided` data
        let next_valid = self
            .current_working_targets
            .iter()
            .skip_while(|&(_k, v)| self.mergers[*v].generated_message.is_empty())
            .next();
        if let Some((_to, pe_id)) = next_valid {
            let pe = &mut self.mergers[*pe_id];
            let mut data = Vec::with_capacity(self.max_data_provided);
            while data.len() < self.max_data_provided && !pe.generated_message.is_empty() {
                data.push(pe.generated_message.pop_front().unwrap());
            }
            data
        } else {
            Vec::new()
        }
    }

    fn peek_data(
        &self,
        _context: &Self::SimContext,
        _current_cycle: u64,
    ) -> Vec<&Self::OutputData> {
        let next_valid = self
            .current_working_targets
            .iter()
            .skip_while(|&(_k, v)| self.mergers[*v].generated_message.is_empty())
            .map(|(_, v)| {
                self.mergers[*v]
                    .generated_message
                    .iter()
                    .take(self.max_data_provided)
                    .collect()
            })
            .next();
        if let Some(next_valic) = next_valid {
            next_valic
        } else {
            vec![]
        }
    }
}

impl<LevelType, Child> Component for SimpleStreamMerger<LevelType, Child>
where
    LevelType: LevelTrait,
    Child: Component<SimContext = SimulationContext<LevelType>>
        + StreamProvider<OutputData = StreamMessage, SimContext = SimulationContext<LevelType>>,
{
    type SimContext = SimulationContext<LevelType>;

    /// - will fetch data from children
    /// - will try to perform merge for each pe
    fn cycle(&mut self, context: &mut Self::SimContext, current_cycle: u64) {
        // 1. first fetch data for children
        for (child_id, child) in self.children.iter_mut().enumerate() {
            child.cycle(context, current_cycle);
            let data = child.peek_data(context, current_cycle);
            // if data cannot be accepted, skip the data
            if Self::can_receive_data(
                &self.mergers,
                child_id,
                &data,
                &self.current_working_targets,
            ) {
                // accept the data
                let data = child.get_data(context, current_cycle);
                Self::receive_data(
                    &mut self.mergers,
                    data,
                    child_id,
                    &mut self.current_working_targets,
                );
            }
        }

        // 2. perform merge for each pe
        for (pe_id, pe) in self.mergers.iter_mut().enumerate() {
            if pe.cycle(context, current_cycle) {
                // the pe is finished and turned to be idle,change the status
                self.current_working_targets.remove(&pe.current_target);
                self.free_pes.insert(pe_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::vec;

    use sprs::{num_kinds::Pattern, CsMat};

    use crate::pim::{level::ddr4, stream_merger::bank_provider::Provider, task::PathId};

    use super::*;
    #[test]
    fn test_simple_stream_merger() {
        let config = Config::from_ddr4(2, 2, 2);
        let graph_b = CsMat::new((2, 2), vec![0, 1, 2], vec![0, 1], vec![Pattern; 2]);

        let children = vec![Provider::<ddr4::Level>::new(2, 2, 2, 2, 2, 2, &graph_b); 2];
        let mut context = SimulationContext::new(&config);
        let mut merger =
            SimpleStreamMerger::new(&config, children, ddr4::Level::BankGroup, 1, 4, 4, 4, 4, 4);
        let path = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0, 0));
        let task = context.gen_task(TaskType::Real, path, 0, 0, 0);
        let mut current_cycle = 0;
        merger
            .receive_task(&task, &mut context, current_cycle)
            .unwrap();
        let message = loop {
            let message = merger.get_data(&mut context, current_cycle);
            if !message.is_empty() {
                break message;
            }
            merger.cycle(&mut context, current_cycle);
            current_cycle += 1;
        };
        println!("{:?}", message);
    }
}
