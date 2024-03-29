use std::fmt::Debug;

use super::level::LevelTrait;

/// a trait that can represent a path to a Bank
#[derive(Debug, Clone)]
pub struct PathId<LevelType: LevelTrait> {
    pub level_path: LevelType::Storage,
}

impl<LevelType: LevelTrait> PathId<LevelType> {
    pub fn new(path: impl Into<LevelType::Storage>) -> Self {
        Self {
            level_path: path.into(),
        }
    }
    pub fn get_level_id(&self, level: &LevelType) -> usize {
        level.get_level_id(&self.level_path)
    }
    pub fn get_row_id(&self) -> usize {
        LevelType::row().get_level_id(&self.level_path)
    }
}

/// the data structure that represents a task
#[derive(Debug, Clone)]
pub struct TaskData<LevelType: LevelTrait> {
    /// unique id for a task
    pub id: usize,
    /// the path to where the row_b is stored
    pub target_id: PathId<LevelType>,
    /// the row id in the matrix B
    pub from: usize,
    /// the row id in the matrix A
    pub to: TaskTo,
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
    pub to: TaskTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskTo {
    pub to: usize,
    pub round: usize,
}

/// the task send to bank
#[derive(Debug, Clone, enum_as_inner::EnumAsInner)]
pub enum Task<LevelType: LevelTrait> {
    /// a real task(an element in A, will find  a row in B)
    TaskData(TaskData<LevelType>),
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
    pub fn gen_task<LevelType: LevelTrait>(
        &mut self,
        target_id: PathId<LevelType>,
        from: usize,
        to: TaskTo,
        size: usize,
    ) -> TaskData<LevelType> {
        let id = self.current_id;
        self.current_id += 1;
        TaskData {
            id,
            target_id,
            from,
            to,
            size,
        }
    }
    /// generate an end signal
    pub fn gen_end_task(&mut self, to: TaskTo) -> TaskEndData {
        let id = self.current_id;
        self.current_id += 1;
        TaskEndData { id, to }
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
    pub to: TaskTo,
    /// the msg type
    pub message_type: StreamMessageType,
}
#[derive(Debug, Default)]
pub struct StreamMessageBuilder {
    current_id: usize,
}

impl StreamMessageBuilder {
    /// generate a real msg
    pub fn generate_msg(&mut self, to: TaskTo, idx: usize, generated_cycle: u64) -> StreamMessage {
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
    pub fn generate_end_msg(&mut self, to: TaskTo) -> StreamMessage {
        let id = self.current_id;
        self.current_id += 1;
        StreamMessage {
            id,
            to,
            message_type: StreamMessageType::End,
        }
    }
}
