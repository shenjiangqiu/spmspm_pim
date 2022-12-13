//! the status of a rowbuffer

/// a row in bank
#[derive(Debug, Clone)]
pub struct Row {
    pub row_id: usize,
    pub data_accessed: usize,
}
/// a bank
#[derive(Debug, Clone, Default)]
pub struct BankState {
    opened_row: Option<Row>,
}

impl BankState {
    /// create a new bank
    pub fn new() -> Self {
        Default::default()
    }

    /// is row opened with `row_id`
    pub fn is_row_ready(&self, row_id: usize) -> bool {
        match self.opened_row {
            Some(ref row) => row.row_id == row_id,
            None => false,
        }
    }

    pub fn is_row_opened(&self) -> bool {
        self.opened_row.is_some()
    }

    /// open row
    pub fn open_row(&mut self, row_id: usize) {
        self.opened_row = Some(Row {
            row_id,
            data_accessed: 0,
        });
    }
}
