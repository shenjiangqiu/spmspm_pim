use std::fmt::Debug;

use super::level::{LevelTrait, PathStorage};

/// a trait that can represent a path to a Bank
#[derive(Debug, Clone)]
pub struct PathId<Storage> {
    level_path: Storage,
}

impl<Storage> PathId<Storage>
where
    Storage: PathStorage,
{
    pub fn new(path: impl Into<Storage>) -> Self {
        Self {
            level_path: path.into(),
        }
    }
    pub fn get_level_id(&self, level: &Storage::LevelType) -> usize {
        self.level_path.get_level_id(level)
    }
    pub fn get_row_id(&self) -> usize {
        self.level_path.get_level_id(&Storage::LevelType::row())
    }
}

/// the data structure that represents a task
#[derive(Debug, Clone)]
pub struct TaskData<Storage> {
    /// unique id for a task
    pub id: usize,
    /// the path to where the row_b is stored
    pub target_id: PathId<Storage>,
    /// the row id in the matrix B
    pub from: usize,
    /// the row id in the matrix A
    pub to: usize,
    /// the size of the row of b in bytes
    pub size: usize,
}

/// the data representation an end signal of a task
/// - it means the tasks from a row of A is ended
#[derive(Debug, Clone)]
pub struct TaskEndData {
    /// the id of the task
    pub id: usize,
    /// the row of A
    pub to: usize,
}

/// the task send to bank
#[derive(Debug, Clone)]
pub enum Task<Storage> {
    /// a real task(an element in A, will find  a row in B)
    TaskData(TaskData<Storage>),
    /// a signal that the tasks from a row of A is ended
    End(TaskEndData),
}

/// task generator
#[derive(Debug, Default)]
pub struct TaskBuilder {
    current_id: usize,
}
impl TaskBuilder {
    // pub fn new() -> Self {
    //     Self::default()
    // }

    /// generate a task
    pub fn gen_task<Storage>(
        &mut self,
        target_id: PathId<Storage>,
        from: usize,
        to: usize,
        size: usize,
    ) -> Task<Storage> {
        let id = self.current_id;
        self.current_id += 1;
        Task::TaskData(TaskData {
            id,
            target_id,
            from,
            to,
            size,
        })
    }
    /// generate an end signal
    pub fn gen_end_task<Storage>(&mut self, to: usize) -> Task<Storage> {
        let id = self.current_id;
        self.current_id += 1;
        Task::End(TaskEndData { id, to })
    }
}

/// a message come from lower level to higher level, which represent a single data of final row or partial sum
#[derive(Debug, Clone)]
pub struct StreamMessageData {
    pub idx: usize,
    pub generated_cycle: u64,
    pub consumed_cycle: u64,
}

/// the message type
#[derive(Debug, Clone)]
pub enum StreamMessageType {
    /// a real message
    Data(StreamMessageData),
    /// a signal that means the stream of a source is ended
    /// - it might be a row of B
    /// - it might be a partial sum from a single source
    End,
}

/// the message send to stream merger
#[derive(Debug, Clone)]
pub struct StreamMessage {
    /// the id of msg
    pub id: usize,
    /// the row_id of A
    pub to: usize,
    /// the msg type
    pub message_type: StreamMessageType,
}
#[derive(Debug, Default)]
pub struct StreamMessageBuilder {
    current_id: usize,
}

impl StreamMessageBuilder {
    /// generate a real msg
    pub fn generate_msg(&mut self, to: usize, idx: usize, generated_cycle: u64) -> StreamMessage {
        let id = self.current_id;
        self.current_id += 1;
        StreamMessage {
            id,
            to,
            message_type: StreamMessageType::Data(StreamMessageData {
                idx,
                generated_cycle,
                consumed_cycle: 0,
            }),
        }
    }
    /// generate an end signal
    pub fn generate_end_msg(&mut self, to: usize) -> StreamMessage {
        let id = self.current_id;
        self.current_id += 1;
        StreamMessage {
            id,
            to,
            message_type: StreamMessageType::End,
        }
    }
}
