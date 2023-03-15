//! the event module

/// An event which have a type and a finished time.
pub struct Event<EventType> {
    /// the finished time of the event
    pub finished_time: u64,
    /// the type of the event
    pub event: EventType,
}

impl<EventType> PartialEq for Event<EventType> {
    fn eq(&self, other: &Self) -> bool {
        self.finished_time == other.finished_time
    }
}
impl<EventType> Eq for Event<EventType> {}
impl<EventType> PartialOrd for Event<EventType> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<EventType> Ord for Event<EventType> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.finished_time.cmp(&self.finished_time)
    }
}
