pub mod provider;
mod simple_stream_merger;
pub use simple_stream_merger::SimpleStreamMerger;
/// receive streamed input from lower layers and merge it and send it to upper layers. it will also send the tasks to the lower layers.
pub trait StreamMerger {
    /// the mutable context shared by all components.
    type SimContext;
    /// send task to lower layer and record the task
    fn process_task(&mut self, context: &mut Self::SimContext, current_cycle: usize);
    /// fetch input and merge it and send it to upper layer
    fn process_input(&mut self, context: &mut Self::SimContext, current_cycle: usize);
}
pub trait EmptyComponent {
    fn is_empty(&self) -> Result<(), String>;
}
/// can provide data from a single output port
pub trait StreamProvider {
    /// the output data provided
    type OutputData;
    /// the mutable context shared by all components.
    type SimContext;

    /// get the data from the output port
    fn get_data(
        &mut self,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Vec<Self::OutputData>;
    /// peek the data from the output port
    fn peek_data(&self, context: &Self::SimContext, current_cycle: u64) -> Vec<&Self::OutputData>;
}

/// can receive a task from a single input port
pub trait TaskReceiver {
    /// the type of the task
    type InputTask;
    /// the mutable context shared by all components.
    type SimContext;
    /// the dram spec
    type LevelType;

    /// receive a task from the input port
    /// - return the level where  the queue is full
    fn receive_task(
        &mut self,
        task: &Self::InputTask,
        context: &mut Self::SimContext,
        current_cycle: u64,
    ) -> Result<(), Self::LevelType>;
}
