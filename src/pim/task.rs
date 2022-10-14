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

#[derive(Debug, Clone)]
pub enum TaskType {
    Real,
    End,
}

#[derive(Debug, Clone)]
pub struct TaskData<Storage> {
    pub id: usize,
    pub target_id: PathId<Storage>,
    pub from: usize,
    pub to: usize,
    pub size: usize,
}
#[derive(Debug, Clone)]
pub struct TaskEndData {
    pub id: usize,
    pub to: usize,
}

/// the task send to bank
#[derive(Debug, Clone)]
pub enum Task<Storage> {
    TaskData(TaskData<Storage>),
    End(TaskEndData),
}
impl<Storage> Task<Storage> {}

#[derive(Debug, Default)]
pub struct TaskBuilder {
    current_id: usize,
}
impl TaskBuilder {
    pub fn new() -> Self {
        Self { current_id: 0 }
    }

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

    pub fn gen_end_task<Storage>(&mut self, to: usize) -> Task<Storage> {
        let id = self.current_id;
        self.current_id += 1;
        Task::End(TaskEndData { id, to })
    }
}

#[derive(Debug, Clone)]
pub struct StreamMessageData {
    pub idx: usize,
    pub generated_cycle: u64,
    pub consumed_cycle: u64,
}

#[derive(Debug, Clone)]
pub enum StreamMessageType {
    Data(StreamMessageData),
    End,
}

/// the message send to stream merger
#[derive(Debug, Clone)]
pub struct StreamMessage {
    pub id: usize,
    pub to: usize,
    pub message_type: StreamMessageType,
}
#[derive(Debug, Default)]
pub struct StreamMessageBuilder {
    current_id: usize,
}

impl StreamMessageBuilder {
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
