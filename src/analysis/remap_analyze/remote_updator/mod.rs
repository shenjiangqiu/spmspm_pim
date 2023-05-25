//! ## rust module
//! ## Author: Jiangqiu Shen
//! ## Date: 2023-05-23
//! Description: the remote updator is used to define how to update the remote dense result

use super::row_cycle::RowLocation;
pub mod selective;
use std::borrow::Borrow;
// pub mod sequential;
/// ## rust function
/// ## Author: Jiangqiu Shen
/// ## Date: 2023-05-23
/// Description: the remote updator is used to define how to update the remote dense result
pub trait RemoteUpdator {
    /// ## rust function
    /// ## Author: Jiangqiu Shen
    /// ## Date: 2023-05-23
    /// Description: update the remote dense result according to input stream, return the cycles
    fn update<Item: Borrow<RowLocation>>(&mut self, data: impl IntoIterator<Item = Item>);
}
