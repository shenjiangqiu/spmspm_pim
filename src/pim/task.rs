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
/// the task send to bank
#[derive(Debug, Clone)]
pub struct Task<Storage> {
    pub id: usize,
    pub target_id: PathId<Storage>,
    pub from: usize,
    pub to: usize,
    pub size: usize,
}

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
        Task {
            id,
            target_id,
            from,
            to,
            size,
        }
    }
}

/// the message send to stream merger
#[derive(Debug, Clone)]
pub struct StreamMessage {
    pub id: usize,
    pub from: usize,
    pub to: usize,
    pub generated_cycle: u64,
    pub consumed_cycle: u64,
}
#[derive(Debug, Default)]
pub struct StreamMessageBuilder {
    current_id: usize,
}

impl StreamMessageBuilder {
    pub fn gen_message_from_task<Storage>(
        &mut self,
        task: &Task<Storage>,
        generated_cycle: u64,
    ) -> StreamMessage {
        let id = self.current_id;
        self.current_id += 1;
        StreamMessage {
            id,
            from: task.from,
            to: task.to,
            generated_cycle,
            consumed_cycle: 0,
        }
    }
}

impl StreamMessage {
    pub fn from_task<Storage>(
        task: Task<Storage>,
        builder: &mut StreamMessageBuilder,
        current_cycle: u64,
    ) -> Self {
        let id = builder.current_id;
        builder.current_id += 1;
        Self {
            id,
            from: task.from,
            to: task.to,
            generated_cycle: current_cycle,
            consumed_cycle: 0,
        }
    }
}
