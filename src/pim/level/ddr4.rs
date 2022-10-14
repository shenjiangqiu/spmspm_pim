use sprs::{num_kinds::Pattern, CsMat};

use super::{LevelTrait, PathStorage};
#[derive(enum_as_inner::EnumAsInner, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Channel = 0,
    Rank,
    Chip,
    BankGroup,
    Bank,
    SubArray,
    Row,
    Column,
}
#[derive(Debug, Clone)]
pub struct Storage {
    pub data: [usize; 8],
}

impl From<[usize; 8]> for Storage {
    fn from(data: [usize; 8]) -> Self {
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
        sub_array: usize,
        row: usize,
        column: usize,
    ) -> Self {
        Self {
            data: [
                channel, rank, chip, bank_group, bank, sub_array, row, column,
            ],
        }
    }

    pub fn get_total_subarrays(&self) -> usize {
        self.data[0] * self.data[1] * self.data[2] * self.data[3] * self.data[4] * self.data[5]
    }
    pub fn forward_to_next_subarray(&mut self, total_size: &Self) {
        self.data[5] += 1;

        for i in 5..=1 {
            if self.data[i] < total_size.data[i] {
                break;
            } else {
                self.data[i] = 0;
                self.data[i - 1] += 1;
            }
        }
        if self.data[0] >= total_size.data[0] {
            self.data[0] = 0;
        }
    }

    pub fn get_flat_subarray_id(&self, total_size: &Self) -> usize {
        let mut id = 0;
        let mut base = 1;
        for i in 5..=0 {
            id += self.data[i] * base;
            base *= total_size.data[i];
        }
        id
    }
}
impl PathStorage for Storage {
    type LevelType = Level;

    fn get_level_id(&self, level: &Self::LevelType) -> usize {
        return self.data[level.to_usize()];
    }
}

pub struct Mapping {
    pub rows: Vec<super::GraphBRow<Storage>>,
}

impl super::MatrixBMapping for Mapping {
    type Storage = Storage;

    fn get_mapping(total_size: &Self::Storage, graph: &CsMat<Pattern>) -> Self {
        let mut current_path = Storage::new(0, 0, 0, 0, 0, 0, 0, 0);
        let mut row_start = vec![(0, 0); total_size.get_total_subarrays()];
        let mut rows = vec![];
        for row in graph.outer_iterator() {
            let size = row.nnz() * 4;
            let current_subarray = current_path.get_flat_subarray_id(total_size);
            let current_start = &mut row_start[current_subarray];
            let mut path = current_path.clone();
            path.data[6] = current_start.0;
            path.data[7] = current_start.1;

            // move forward the next start
            current_start.1 += size;
            while current_start.1 >= total_size.data[7] {
                current_start.0 += 1;
                current_start.1 -= total_size.data[7];
            }
            // forward to next subarray

            let graph_b_row = super::GraphBRow { path, size };
            rows.push(graph_b_row);
        }
        Mapping { rows }
    }

    fn get_row_detail(&self, row: usize) -> &super::GraphBRow<Self::Storage> {
        &self.rows[row]
    }
}

impl LevelTrait for Level {
    const LEVELS: usize = 7;

    type Storage = Storage;
    type Mapping = Mapping;
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
            Level::Bank => Some(Level::SubArray),
            Level::SubArray => Some(Level::Row),
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
