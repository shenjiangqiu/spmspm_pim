use crate::pim::level::LevelTrait;

pub struct Event<EventType> {
    pub finished_time: u64,
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
