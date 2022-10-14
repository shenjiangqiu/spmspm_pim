/// a row in bank
#[derive(Debug, Clone)]
pub struct Row {
    pub row_id: usize,
    pub data_accesed: usize,
}
/// a bank
#[derive(Debug, Clone)]
pub struct BankState {
    opened_row: Option<Row>,
}

impl BankState {
    pub fn new() -> Self {
        Self { opened_row: None }
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
            data_accesed: 0,
        });
    }
}
