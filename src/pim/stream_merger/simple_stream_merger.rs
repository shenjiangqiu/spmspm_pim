use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Debug,
};

use enum_as_inner::EnumAsInner;
use tracing::debug;

use crate::pim::{
    config::Config,
    level::LevelTrait,
    task::{self, StreamMessage, StreamMessageData, Task, TaskTo},
    Component, SimulationContext,
};

use super::{EmptyComponent, StreamProvider, TaskReceiver};

#[derive(Debug, Clone, EnumAsInner)]
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
    id: usize,
    status: Waiting,
    max_buffered_message: usize,
    max_generated_message: usize,
    max_stored_generated_message: usize,
    /// the data that finished processing
    generated_message: VecDeque<StreamMessage>,
    pub current_target: TaskTo,
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
    fn cycle<LevelType: LevelTrait>(
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
                            debug!(self.id, "merger {} generated msg: {:?}", self.id, msg);
                            self.generated_message.push_back(msg);
                        }
                        // no message generated, means that all input is empty
                        None => {
                            // generate a end message if all input is empty and switch to WaitingForDrain
                            let end_message = context.generate_end(self.current_target);
                            self.generated_message.push_back(end_message);
                            self.status = Waiting::WaitingForDrain;
                            debug!(self.id, "merger {} switch to WaitingForDrain", self.id);
                            return false;
                        }
                    }
                }
                false
                // all data received
            }
            Waiting::WaitingForDrain => {
                //  decide if all data is drained
                if self.generated_message.is_empty() {
                    self.status = Waiting::Idle;
                    true
                } else {
                    false
                }
            }
            _ => false,
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

            Some(min_idx)
        } else {
            None
        }
    }
}

impl MergerStatus {
    pub fn new(
        id: usize,
        num_child: usize,
        max_buffered_message: usize,
        max_generated_message: usize,
        max_stored_generated_message: usize,
    ) -> Self {
        Self {
            id,
            status: Waiting::Idle,
            max_buffered_message,
            current_target: TaskTo { to: 0, round: 0 },
            waiting_data_childs: BTreeSet::new(),
            buffered_message: vec![VecDeque::with_capacity(max_buffered_message); num_child],
            max_generated_message,
            max_stored_generated_message,
            generated_message: VecDeque::with_capacity(max_stored_generated_message),
        }
    }
}

impl MergerStatus {
    pub fn receive_task(&mut self, to: &TaskTo, child_id: usize, _num_children: usize) {
        match self.status {
            Waiting::Idle => {
                self.status = Waiting::WaitingForTask;
                self.current_target = *to;
                self.waiting_data_childs.insert(child_id);
            }
            Waiting::WaitingForTask => {
                if &self.current_target != to {
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
    id: usize,
    /// the level of this merger
    current_level: LevelType,
    /// the children of this merger
    children: Vec<Child>,
    /// the status of each merger
    mergers: Vec<MergerStatus>,
    /// current receiving targets, target_id to pe_id
    current_receiving_targets: BTreeMap<TaskTo, usize>,
    /// current working targets, target_id to pe_id
    current_working_targets: BTreeMap<TaskTo, usize>,
    /// free pes
    free_pes: BTreeSet<usize>,
}
impl<LevelType: Debug, Child> EmptyComponent for SimpleStreamMerger<LevelType, Child>
where
    Child: EmptyComponent,
{
    /// a test util
    fn is_empty(&self) -> Vec<String> {
        let mut result = vec![];
        if !self.current_receiving_targets.is_empty() {
            result.push(format!(
                "current_receiving_targets is not empty: {:?}",
                self.current_receiving_targets
            ));
        }
        if !self.current_working_targets.is_empty() {
            result.push(format!(
                "level: {:?}, id: {:?}, current_working_targets is not empty: {:?}",
                self.current_level, self.id, self.current_working_targets
            ));
        }
        if !self.free_pes.len() == self.mergers.len() {
            result.push(format!("free_pes is not full: {:?}", self.free_pes));
        }
        for merger in self.mergers.iter() {
            if !merger.status.is_idle() {
                result.push(format!("merger {} is not idle", merger.id));
            }
            if !merger.waiting_data_childs.is_empty() {
                result.push(format!(
                    "merger {} waiting_data_childs is not empty",
                    merger.id
                ));
            }
            if !merger.buffered_message.iter().all(|v| v.is_empty()) {
                result.push(format!(
                    "merger {} buffered_message is not empty",
                    merger.id
                ));
            }
            if !merger.generated_message.is_empty() {
                result.push(format!(
                    "merger {} generated_message is not empty",
                    merger.id
                ));
            }
        }

        for child in self.children.iter() {
            result.extend(child.is_empty());
        }
        result
    }
}

impl<LevelType: LevelTrait + Debug, Child> SimpleStreamMerger<LevelType, Child> {
    #[allow(clippy::too_many_arguments)]
    /// create a new simple stream merger
    pub fn new(
        id: usize,
        _config: &Config,
        children: Vec<Child>,
        level: LevelType,
        num_merger: usize,
        max_buffered_message: usize,
        max_generated_message: usize,
        max_stored_generated_message: usize,
    ) -> Self {
        let num_child = children.len();
        Self {
            id,
            current_level: level,
            children,
            mergers: (0..num_merger)
                .map(|id| {
                    MergerStatus::new(
                        id,
                        num_child,
                        max_buffered_message,
                        max_generated_message,
                        max_stored_generated_message,
                    )
                })
                .collect(),
            current_receiving_targets: BTreeMap::new(),
            current_working_targets: BTreeMap::new(),
            free_pes: (0..num_merger).collect(),
        }
    }

    fn can_receive_data(
        merger_status: &[MergerStatus],
        child_id: usize,
        data: &[&StreamMessage],
        current_working_map: &BTreeMap<TaskTo, usize>,
        current_receiving_map: &BTreeMap<TaskTo, usize>,
    ) -> bool {
        for i in data {
            let to = i.to;
            if let Some(&pe_id) = current_working_map.get(&to) {
                let merger = &merger_status[pe_id];
                if merger.buffered_message[child_id].len() >= merger.max_buffered_message {
                    // the buffer is full, can not receive data
                    return false;
                }
            } else {
                // the task is not ready, cannot receive data
                // the task should be receiving the task set
                assert!(current_receiving_map.contains_key(&to));
                return false;
            }
        }
        // all data can be received
        true
    }
    fn receive_data(
        merger_status: &mut [MergerStatus],
        data: impl IntoIterator<Item = StreamMessage>,
        child_id: usize,
        current_working_map: &mut BTreeMap<TaskTo, usize>,
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
                    let removed = merger.waiting_data_childs.remove(&child_id);
                    assert!(removed);
                }
            }
        }
    }

    /// should not be used by user,
    /// use `receive_task` instead
    fn can_self_receive_task(&self, target_id: &TaskTo) -> bool {
        self.current_receiving_targets.contains_key(target_id) || !self.free_pes.is_empty()
    }
    /// should not be used by user,
    /// use `receive_task` instead
    /// ### **only use this after `can_self_receive_task` returns true**
    fn self_receive_task(&mut self, to: TaskTo, child_id: usize) {
        // yes we can receive this task

        let entry = self.current_receiving_targets.entry(to).or_insert_with(|| {
            let &pe_id = self.free_pes.iter().next().unwrap();
            self.free_pes.remove(&pe_id);
            pe_id
        });
        let pe = &mut self.mergers[*entry];
        pe.receive_task(&to, child_id, self.children.len());
    }

    /// receive a end task
    fn self_receive_end(&mut self, to: &TaskTo) {
        if let Some(pe_id) = self.current_receiving_targets.remove(to) {
            let pe = &mut self.mergers[pe_id];
            pe.receive_end();
            self.current_working_targets.insert(*to, pe_id);
        }
    }
}

impl<LevelType, Child> TaskReceiver for SimpleStreamMerger<LevelType, Child>
where
    Child: TaskReceiver<
        InputTask = Task<LevelType>,
        SimContext = SimulationContext<LevelType>,
        LevelType = LevelType,
    >,
    LevelType: LevelTrait,
{
    type InputTask = Task<LevelType>;
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
        task: Self::InputTask,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Result<(), (Self::LevelType, Self::InputTask)> {
        // recursively receive task
        //

        // check if self can accept the task
        match task {
            Task::TaskData(ref task_data) => {
                if !self.can_self_receive_task(&task_data.to) {
                    debug!(?self.current_level,
                        self.id,
                        "merger can not receive task,level: {:?}", self.current_level
                    );
                    return Err((self.current_level, task));
                }
            }
            Task::End(ref _end_task) => {
                // yes we can
            }
        }
        // send task to lower pe and record the task
        let child_level = self
            .current_level
            .get_child_level()
            .expect("no child level");

        match task {
            Task::TaskData(ref task_data) => {
                debug!(?self.current_level,
                    self.id,
                    "receive task,path: {:?}",task_data.target_id,
                );
                let child_id = task_data.target_id.get_level_id(&child_level);
                // first test if self can receive this task
                debug!(?self.current_level,self.id, "test child:{:?}-{}", child_level, child_id,);
                let to = task_data.to;
                self.children[child_id].receive_task(task, context, current_cycle)?;
                // record this task
                debug!(?self.current_level,
                    self.id,
                    "merger receive task,level: {:?},target: {:?}",
                    self.current_level,
                    to
                );
                self.self_receive_task(to, child_id);
                Ok(())
            }
            Task::End(ref end_data) => {
                // broadcast to all child
                let to = end_data.to;
                if let Some(working_pe_id) = self.current_receiving_targets.get(&to) {
                    let working_pe = &mut self.mergers[*working_pe_id];
                    for child in working_pe.waiting_data_childs.iter() {
                        self.children[*child]
                            .receive_task(task.clone(), context, current_cycle)
                            .unwrap();
                    }
                }

                self.self_receive_end(&to);
                Ok(())
            }
        }
    }
}

impl<LevelType: Debug, Child> StreamProvider for SimpleStreamMerger<LevelType, Child> {
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
            .find(|&(_k, v)| !self.mergers[*v].generated_message.is_empty());
        if let Some((_to, pe_id)) = next_valid {
            let pe = &mut self.mergers[*pe_id];
            let mut data = Vec::new();
            while data.len() < pe.max_generated_message && !pe.generated_message.is_empty() {
                let message = pe.generated_message.pop_front().unwrap();
                debug!(?self.current_level,self.id,
                    "StreamProvider(StringMerger) pop data from pe {}, to: {:?} in level: {:?},msg: {:?}",
                    pe_id, _to, self.current_level, message
                );
                data.push(message);
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
                let pe = &self.mergers[*v];
                let max_generated_message = pe.max_generated_message;

                pe.generated_message
                    .iter()
                    .take(max_generated_message)
                    .collect()
            })
            .next();
        if let Some(next_valic) = next_valid {
            next_valic
        } else {
            Default::default()
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
                &self.current_receiving_targets,
            ) {
                // accept the data
                let data = child.get_data(context, current_cycle);
                Self::receive_data(
                    &mut self.mergers,
                    data,
                    child_id,
                    &mut self.current_working_targets,
                );
            } else {
                // cannot receive the data! the data must be in the current_receiving_targets
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

    use crate::{
        init_logger_debug,
        pim::{config::LevelConfig, level::ddr4, stream_merger::provider::Provider, task::PathId},
    };

    use super::*;

    #[test]
    fn test_simple_stream_merger() {
        init_logger_debug();
        debug!("test_simple_stream_merger");
        let config = Config::from_ddr4_3200(
            LevelConfig {
                num: 1,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            LevelConfig {
                num: 2,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
        );
        let graph_b = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();

        let children = vec![
            Provider::<ddr4::Level>::new(0, 2, 2, 2, 2, 2, &graph_b),
            Provider::<ddr4::Level>::new(1, 2, 2, 2, 2, 2, &graph_b),
        ];
        let mut context = SimulationContext::new(&config);
        let mut merger =
            SimpleStreamMerger::new(0, &config, children, ddr4::Level::Bank, 2, 4, 4, 4);
        let path = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0, 0));
        let task = context.gen_task(path, 0, TaskTo { to: 0, round: 0 }, 4);
        let current_cycle = 0;
        merger
            .receive_task(Task::TaskData(task), &mut context, current_cycle)
            .unwrap();
        let end_task = context.gen_end_task(TaskTo { to: 0, round: 0 });
        merger
            .receive_task(Task::End(end_task), &mut context, current_cycle)
            .unwrap();
        let mut data = vec![];
        for i in 0..100 {
            merger.cycle(&mut context, i);
            let result = merger.get_data(&mut context, i);
            data.extend(result);
        }
        for i in data {
            println!("{:?}", i);
        }
    }

    /// test task and message pass from multiple level
    #[test]
    fn test_2_merger_2_provider() {
        init_logger_debug();
        debug!("test_simple_stream_merger");
        let config = Config::from_ddr4_3200(
            LevelConfig {
                num: 1,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            LevelConfig {
                num: 2,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
        );
        let graph_b = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();

        let providers: Vec<_> = (0..2)
            .map(|id| Provider::<ddr4::Level>::new(id, 2, 2, 2, 2, 2, &graph_b))
            .collect();
        let bank_mergers = (0..2)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    &config,
                    providers.clone(),
                    ddr4::Level::Bank,
                    2,
                    4,
                    4,
                    4,
                )
            })
            .collect();
        let mut bg_merger =
            SimpleStreamMerger::new(0, &config, bank_mergers, ddr4::Level::BankGroup, 2, 4, 4, 4);

        let mut context = SimulationContext::new(&config);

        let path = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0, 0));
        let task = context.gen_task(path, 0, TaskTo { to: 0, round: 0 }, 4);
        let current_cycle = 0;
        bg_merger
            .receive_task(Task::TaskData(task), &mut context, current_cycle)
            .unwrap();
        let end_task = context.gen_end_task(TaskTo { to: 0, round: 0 });
        bg_merger
            .receive_task(Task::End(end_task), &mut context, current_cycle)
            .unwrap();
        let mut data = vec![];
        for i in 0..100 {
            bg_merger.cycle(&mut context, i);
            let result = bg_merger.get_data(&mut context, i);
            data.extend(result);
        }
        for i in data {
            println!("{:?}", i);
        }
        let result = bg_merger.is_empty();
        if !result.is_empty() {
            tracing::error!("{:?}", result);
        }
    }

    /// test multiple task working in parallel
    #[test]
    fn test_2_merger_2_provider_multitask() {
        init_logger_debug();
        debug!("test_simple_stream_merger");
        let config = Config::from_ddr4_3200(
            LevelConfig {
                num: 1,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            LevelConfig {
                num: 2,
                merger_num: 10,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
        );
        let graph_b = sprs::io::read_matrix_market("mtx/test.mtx")
            .unwrap()
            .to_csr();

        let providers: Vec<_> = (0..2)
            .map(|id| Provider::<ddr4::Level>::new(id, 2, 4, 2, 2, 2, &graph_b))
            .collect();
        let bank_mergers = (0..2)
            .map(|id| {
                SimpleStreamMerger::new(
                    id,
                    &config,
                    providers.clone(),
                    ddr4::Level::Bank,
                    2,
                    4,
                    4,
                    4,
                )
            })
            .collect();
        let mut bg_merger =
            SimpleStreamMerger::new(0, &config, bank_mergers, ddr4::Level::BankGroup, 2, 4, 4, 4);

        let mut context = SimulationContext::new(&config);

        let path1 = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 0, 0, 0, 0));
        let path2 = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 0, 1, 0, 0));
        let path3 = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 1, 0, 0, 0));
        let path4 = PathId::new(ddr4::Storage::new(0, 0, 0, 0, 1, 1, 0, 0));
        let task1 = context.gen_task(path1, 0, TaskTo { to: 0, round: 0 }, 4);
        let task2 = context.gen_task(path2, 0, TaskTo { to: 0, round: 0 }, 4);
        let task3 = context.gen_task(path3, 0, TaskTo { to: 1, round: 0 }, 4);
        let task4 = context.gen_task(path4, 0, TaskTo { to: 1, round: 0 }, 4);

        let current_cycle = 0;
        bg_merger
            .receive_task(Task::TaskData(task1), &mut context, current_cycle)
            .unwrap();
        bg_merger
            .receive_task(Task::TaskData(task2), &mut context, current_cycle)
            .unwrap();
        bg_merger
            .receive_task(Task::TaskData(task3), &mut context, current_cycle)
            .unwrap();
        bg_merger
            .receive_task(Task::TaskData(task4), &mut context, current_cycle)
            .unwrap();
        let end_task1 = context.gen_end_task(TaskTo { to: 0, round: 0 });
        let end_task2 = context.gen_end_task(TaskTo { to: 1, round: 0 });
        bg_merger
            .receive_task(Task::End(end_task1), &mut context, current_cycle)
            .unwrap();
        bg_merger
            .receive_task(Task::End(end_task2), &mut context, current_cycle)
            .unwrap();
        let mut data = vec![];
        for i in 0..100 {
            bg_merger.cycle(&mut context, i);
            let result = bg_merger.get_data(&mut context, i);
            data.extend(result);
        }
        for i in data {
            println!("{:?}", i);
        }
        let result = bg_merger.is_empty();
        if !result.is_empty() {
            tracing::error!("{:?}", result);
            panic!();
        }
    }
}
