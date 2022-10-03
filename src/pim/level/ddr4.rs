use super::{LevelTrait, PathStorage};
#[derive(enum_as_inner::EnumAsInner, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Channel = 0,
    Rank,
    Chip,
    BankGroup,
    Bank,
    Row,
    Column,
}
#[derive(Debug, Clone)]
pub struct Storage {
    data: [usize; 7],
}

impl From<[usize; 7]> for Storage {
    fn from(data: [usize; 7]) -> Self {
        Self { data }
    }
}

impl Storage {
    pub fn new(
        channel: usize,
        rank: usize,
        chip: usize,
        bank_group: usize,
        bank: usize,
        row: usize,
        column: usize,
    ) -> Self {
        Self {
            data: [channel, rank, chip, bank_group, bank, row, column],
        }
    }
}
impl PathStorage for Storage {
    type LevelType = Level;

    fn get_level_id(&self, level: &Self::LevelType) -> usize {
        return self.data[level.to_usize()];
    }
}

impl LevelTrait for Level {
    const LEVELS: usize = 7;

    type Storage = Storage;

    fn is_bank(&self) -> bool {
        self.is_bank()
    }
    fn is_channel(&self) -> bool {
        self.is_channel()
    }

    fn is_last(&self) -> bool {
        self.is_column()
    }

    fn get_child_level(&self) -> Option<Self> {
        match self {
            Level::Channel => Some(Level::Rank),
            Level::Rank => Some(Level::Chip),
            Level::Chip => Some(Level::BankGroup),
            Level::BankGroup => Some(Level::Bank),
            Level::Bank => Some(Level::Row),
            Level::Row => Some(Level::Column),
            Level::Column => None,
        }
    }

    fn to_usize(self) -> usize {
        self as usize
    }

    fn first_level() -> Self {
        Self::Channel
    }

    fn last_level() -> Self {
        Self::Bank
    }

    fn row() -> Self {
        Self::Row
    }
}
