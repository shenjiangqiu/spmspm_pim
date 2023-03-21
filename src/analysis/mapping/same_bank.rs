//! Mapping for same bank
//!
//! todo: implement this

use crate::tools;

use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BitsField {
    bits: usize,
    offset: usize,
}

impl BitsField {
    fn get(&self, addr: usize) -> usize {
        let mask = (1 << self.bits) - 1;
        (addr >> self.offset) & mask
    }
}

/// the row and dense col share the same mapping
#[derive(Debug)]
pub struct SameBankMapping {
    subarray_bits: BitsField,
    inner_insubarray_bits: BitsField,
    bank_bits: BitsField,
    channel_bits: BitsField,
    outer_insubarray_bits: BitsField,
}

impl SameBankMapping {
    /// return the row_id of some dense col
    fn get_col_rowid(&self, col_id: usize) -> usize {
        let inner_bits = self.inner_insubarray_bits.get(col_id);
        let outer_bits = self.outer_insubarray_bits.get(col_id);
        let inner_offset = self.inner_insubarray_bits.bits;
        let total_bits = outer_bits << inner_offset | inner_bits;
        // this is the id inside a subarray, we should get the row id from it, each row contains 256/4 = 64 data

        total_bits >> 6
    }

    /// return the col_id of some dense col
    fn get_col_colid(&self, col_id: usize) -> usize {
        let inner_bits = self.inner_insubarray_bits.get(col_id);
        let outer_bits = self.outer_insubarray_bits.get(col_id);
        let inner_offset = self.inner_insubarray_bits.bits;
        let total_bits = outer_bits << inner_offset | inner_bits;
        // this is the id inside a subarray, we should get the row id from it, each row contains 256/4 = 64 data
        // and the bit shif(4 bytes for the data)
        (total_bits & ((1 << 6) - 1)) << 2
    }

    /// return the row_id of some sparse row
    fn get_row_rowid(&self, row_id: usize) -> usize {
        let inner_bits = self.inner_insubarray_bits.get(row_id);
        let outer_bits = self.outer_insubarray_bits.get(row_id);
        let inner_offset = self.inner_insubarray_bits.bits;
        let total_bits = outer_bits << inner_offset | inner_bits;
        // this is already the row id, because each row will occupy the whole row!
        total_bits
    }

    /// return the col_id of some sparse row
    /// no such thing, because the col_id is always 0!!!
    #[allow(dead_code)]
    fn get_row_colid(&self, _row_id: usize) -> usize {
        0
    }

    /// return the global subarray id
    fn get_global_subarray_id(&self, row_id: usize) -> usize {
        let channel_id = self.channel_bits.get(row_id);
        let bank_id = self.bank_bits.get(row_id);
        let subarray_id = self.subarray_bits.get(row_id);
        (channel_id << (self.bank_bits.bits + self.subarray_bits.bits))
            | (bank_id << self.subarray_bits.bits)
            | subarray_id
    }
}

impl SameBankMapping {
    pub fn new(
        total_rows: usize,
        total_banks: usize,
        total_channels: usize,
        total_subarrays: usize,
    ) -> Self {
        let bank_bits = tools::math::count_to_log(total_banks);
        let total_bits = tools::math::count_to_log(total_rows);
        let subarray_bits = tools::math::count_to_log(total_subarrays);
        let channel_bits = tools::math::count_to_log(total_channels);
        let row_bits = total_bits - 1 - bank_bits - subarray_bits - channel_bits;
        // first calculate howmany rows for a banks in average
        let subarray_bits = BitsField {
            bits: subarray_bits,
            offset: 0,
        };
        let inner_insubarray_bits = BitsField {
            bits: row_bits,
            offset: subarray_bits.offset + subarray_bits.bits,
        };
        let bank_bits = BitsField {
            bits: bank_bits,
            offset: inner_insubarray_bits.offset + inner_insubarray_bits.bits,
        };
        let channel_bits = BitsField {
            bits: channel_bits,
            offset: bank_bits.offset + bank_bits.bits,
        };
        let outer_insubarray_bits = BitsField {
            bits: 1,
            offset: channel_bits.offset + channel_bits.bits,
        };

        Self {
            subarray_bits,
            inner_insubarray_bits,
            bank_bits,
            channel_bits,
            outer_insubarray_bits,
        }
    }
}

impl Mapping for SameBankMapping {
    fn get_row_id_evil(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(mat_b_row_id.0)
    }

    fn get_tsv_id_from_subarray(&self, sub_array_id: SubarrayId) -> TsvId {
        //     TsvId(sub_array_id.0 / self.config.subarrays / self.config.banks.num)
        // should be the channel id
        TsvId(sub_array_id.0 >> self.subarray_bits.bits >> self.bank_bits.bits)
    }

    fn get_tsv_id_from_ring(&self, ring_id: RingId) -> TsvId {
        //     TsvId(ring_id.0)
        // ring id is also the channel id
        TsvId(ring_id.0)
    }

    fn ring_port_from_subarray(&self, subarray_id: SubarrayId) -> RingPort {
        //     RingPort(((subarray_id.0 / self.config.subarrays) % self.config.banks.num) as u8)
        // ring port is the relative bank id
        let id = (subarray_id.0 >> self.subarray_bits.bits) & ((1 << self.bank_bits.bits) - 1);
        RingPort(id as u8)
    }

    fn ring_buffer_id(&self, subarray_id: SubarrayId) -> RingBufferId {
        //     RingBufferId(subarray_id.0 / self.config.subarrays)
        // ring buffer id is the absolute bank id
        RingBufferId(subarray_id.0 >> self.subarray_bits.bits)
    }

    fn ring_id_from_subarray(&self, partition_id: SubarrayId) -> RingId {
        // ring id is the channel id
        RingId(partition_id.0 >> self.subarray_bits.bits >> self.bank_bits.bits)
    }

    fn get_row_id(&self, mat_b_row_id: LogicRowId, _col_id: LogicColId) -> PhysicRowId {
        PhysicRowId(self.get_row_rowid(mat_b_row_id.0))
    }

    fn get_row_id_dense(&self, _target_row_id: LogicRowId, col_id: LogicColId) -> PhysicRowId {
        //     let real_col_id =
        //         target_row_id.0 * self.col_per_partition * 4 + col_id.0 % self.col_per_partition;
        //     PhysicRowId(real_col_id / 256)
        // frist calculate howmany bits are used for the entire row of dense addr

        // then shift the row id to the left and plus the row addr.
        // step 1, calculate the row size

        PhysicRowId(self.get_col_rowid(col_id.0))
    }

    fn get_col_id_dense(&self, _target_row_id: LogicRowId, col_id: LogicColId) -> usize {
        self.get_col_colid(col_id.0)
    }

    fn get_partition_id_row(&self, row_id: LogicRowId) -> SubarrayId {
        SubarrayId(self.get_global_subarray_id(row_id.0))
    }

    fn get_partition_id_col(&self, col_id: LogicColId) -> SubarrayId {
        SubarrayId(self.get_global_subarray_id(col_id.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mapping() {
        let mapping = SameBankMapping::new(100, 4, 4, 4);
        assert_eq!(mapping.subarray_bits, BitsField { bits: 2, offset: 0 });
        assert_eq!(
            mapping.inner_insubarray_bits,
            BitsField { bits: 0, offset: 2 }
        );
        assert_eq!(mapping.bank_bits, BitsField { bits: 2, offset: 2 });
        assert_eq!(mapping.channel_bits, BitsField { bits: 2, offset: 4 });
        assert_eq!(
            mapping.outer_insubarray_bits,
            BitsField { bits: 1, offset: 6 }
        );
    }
    #[test]
    fn test_mapping_1000() {
        let mapping = SameBankMapping::new(1000, 4, 4, 4);
        assert_eq!(mapping.subarray_bits, BitsField { bits: 2, offset: 0 });
        assert_eq!(
            mapping.inner_insubarray_bits,
            BitsField { bits: 3, offset: 2 }
        );
        assert_eq!(mapping.bank_bits, BitsField { bits: 2, offset: 5 });
        assert_eq!(mapping.channel_bits, BitsField { bits: 2, offset: 7 });
        assert_eq!(
            mapping.outer_insubarray_bits,
            BitsField { bits: 1, offset: 9 }
        );
    }
    #[test]
    fn test_row_id() {
        let mapping = SameBankMapping::new(1000, 4, 4, 4);
        let row_id = mapping.get_row_rowid(10);
        assert_eq!(row_id, 2);
        let sp_col_id = mapping.get_row_colid(row_id);
        assert_eq!(sp_col_id, 0);

        let dense_row_id = mapping.get_col_rowid(10);
        // still the same row
        assert_eq!(dense_row_id, 0);
        let dense_col_id = mapping.get_col_colid(10);
        // the col id is 2, each data have 4 bytes, so it's 8
        assert_eq!(dense_col_id, 8);

        let dense_row_id = mapping.get_col_rowid(100);
        assert_eq!(dense_row_id, 0);
        let dense_col_id = mapping.get_col_colid(100);
        assert_eq!(dense_col_id, 4);

        // test the outer
        let dense_row_id = mapping.get_col_rowid(999);
        assert_eq!(dense_row_id, 0);
        let dense_col_id = mapping.get_col_colid(999);
        assert_eq!(dense_col_id, 9 * 4);
    }

    #[test]
    fn test_row_id_10000() {
        let mapping = SameBankMapping::new(10000, 4, 4, 4);

        // test the outer
        let dense_row_id = mapping.get_col_rowid(9999);
        assert_eq!(dense_row_id, 3);
        let dense_col_id = mapping.get_col_colid(9999);
        assert_eq!(dense_col_id, 3 * 4);
    }

    #[test]
    fn test_sp_row_id_10000() {
        let mapping = SameBankMapping::new(10000, 4, 4, 4);

        // test the outer
        let dense_row_id = mapping.get_row_rowid(9999);
        assert_eq!(dense_row_id, 195);
    }

    #[test]
    fn test_global_subarray_id() {
        let mapping = SameBankMapping::new(10000, 4, 4, 4);
        let subarray_id = mapping.get_global_subarray_id(9999);
        assert_eq!(subarray_id, 15);
    }
}
