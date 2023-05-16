//! a implementation of spec DDR4
use sprs::{num_kinds::Pattern, CompressedStorage::CSR, CsMat};
use tracing::debug;

use super::LevelTrait;

const LEVELS: usize = 8;

/// the levels of ddr4
#[allow(missing_docs)]
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

/// the storage to store the path
/// - 0: channel - 1: rank - 2: chip - 3: bank group - 4: bank - 5: sub array - 6: row - 7: column
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Storage {
    /// 0: channel - 1: rank - 2: chip - 3: bank group - 4: bank - 5: sub array - 6: row - 7: column
    pub data: [usize; LEVELS],
}

impl From<[usize; LEVELS]> for Storage {
    fn from(data: [usize; LEVELS]) -> Self {
        Self { data }
    }
}

impl Storage {
    #[allow(clippy::too_many_arguments)]
    /// create a new storage
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
    /// - when `self` is the size of the whole ddr4
    /// - it will return the number of subarrays in total
    pub fn get_total_level(&self, level: &Level) -> usize {
        // self.data[0] * self.data[1] * self.data[2] * self.data[3] * self.data[4] * self.data[5]
        self.data[0..=level.to_usize()].iter().product()
    }

    /// - given `self` is a path, `total_size` is the size of the whole ddr4
    /// - it will move `self` to the next subarray
    pub fn forward_to_next_subarray(&mut self, total_size: &Self) {
        self.data[5] += 1;

        // forward from subarray to rank
        for i in (1..=5).rev() {
            if self.data[i] < total_size.data[i] {
                break;
            } else {
                self.data[i] = 0;
                self.data[i - 1] += 1;
            }
        }

        // round up the channel number
        if self.data[0] >= total_size.data[0] {
            self.data[0] = 0;
        }
    }
    /// - given `self` is a path, `total_size` is the size of the whole ddr4
    /// - it return the global subarray id
    pub fn get_flat_level_id(&self, total_size: &Self, level: &Level) -> usize {
        let mut id = 0;
        let mut base = 1;
        for i in (0..=level.to_usize()).rev() {
            id += self.data[i] * base;
            base *= total_size.data[i];
        }
        id
    }
}

/// the Matrix to Dram storage mapping for ddr4
#[derive(Debug)]
pub struct Mapping {
    /// the detailed mapping for each row in matrix
    pub rows: Vec<super::GraphBRow<Storage>>,
}

impl LevelTrait for Level {
    const LEVELS: usize = LEVELS;

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
    /// the last level to receive data
    fn last_level() -> Self {
        Self::SubArray
    }

    fn subarray() -> Self {
        Self::SubArray
    }
    fn row() -> Self {
        Self::Row
    }

    fn bank() -> Self {
        Self::Bank
    }

    fn col() -> Self {
        Self::Column
    }
    fn get_level_id(&self, storage: &Storage) -> usize {
        storage.data[self.to_usize()]
    }

    fn get_sub_path_to_level(&self, storage: &Storage) -> Self::Storage {
        let mut data = [0; LEVELS];
        data[..(self.to_usize() + 1)].copy_from_slice(&storage.data[..(self.to_usize() + 1)]);
        Storage { data }
    }

    fn get_total_level(&self, total_size: &Self::Storage) -> usize {
        total_size.get_total_level(self)
    }

    fn get_flat_level_id(&self, total_size: &Self::Storage, id: &Self::Storage) -> usize {
        id.get_flat_level_id(total_size, self)
    }

    /// TODO: Description
    fn get_mapping(total_size: &Self::Storage, graph: &CsMat<Pattern>) -> Self::Mapping {
        assert_eq!(graph.storage(), CSR);
        debug!(
            "start to build mapping for ddr4,total size: {:?}",
            total_size
        );
        let mut current_path = Storage::new(0, 0, 0, 0, 0, 0, 0, 0);
        let mut row_start = vec![(0, 0); total_size.get_total_level(&Level::last_level())];
        let mut rows = vec![];
        for row in graph.outer_iterator() {
            let size = row.nnz() * 4;
            let current_subarray = current_path.get_flat_level_id(total_size, &Level::last_level());
            let current_start = &mut row_start[current_subarray];
            let mut path = current_path.clone();
            debug_assert!(path.data[5] != total_size.data[5]);
            path.data[6] = current_start.0;
            path.data[7] = current_start.1;

            // move forward the next start
            current_start.1 += size;
            while current_start.1 >= total_size.data[7] {
                current_start.0 += 1;
                current_start.1 -= total_size.data[7];
            }
            // forward to next subarray

            let graph_b_row = super::GraphBRow {
                path,
                size,
                nnz: row.nnz(),
            };
            rows.push(graph_b_row);
            current_path.forward_to_next_subarray(total_size);
        }
        Mapping { rows }
    }

    fn get_row_detail(mapping: &Mapping, row: usize) -> &super::GraphBRow<Storage> {
        &mapping.rows[row]
    }

    fn set_one_to_level(storage: &Self::Storage, level: &Self) -> Self::Storage {
        let mut storage = storage.clone();
        for i in 0..(level.to_usize()) {
            storage.data[i] = 1;
        }
        storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mapping() {
        let graph = sprs::io::read_matrix_market("test_mtx/test.mtx")
            .unwrap()
            .to_csr();
        let total_size = Storage::new(1, 1, 8, 4, 4, 2, 2, 16);
        let mapping = Level::get_mapping(&total_size, &graph);
        println!("{:?}", mapping);
    }

    #[test]
    fn test_storage() {}
}
