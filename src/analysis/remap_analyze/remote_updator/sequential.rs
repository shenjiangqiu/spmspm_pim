use crate::analysis::remap_analyze::{action::UpdateAction, row_cycle::*};

pub struct SequentialRemoteUpdator;

impl super::RemoteUpdator for SequentialRemoteUpdator {
    type Status<'a> = (
        SubarrayId,
        &'a mut AllJumpCycles,
        &'a mut RowIdWordId,
        usize,
    );

    fn update(
        &mut self,
        data: impl IntoIterator<Item = RowIdWordId>,
        status: &mut Self::Status<'_>,
    ) {
        for task in data {
            let loc = RowLocation {
                subarray_id: status.0,
                row_id_word_id: task,
            };
            let mut update_action = UpdateAction {
                row_status: status.2,
                loc: &loc,
                size: WordId(1),
                remap_cycle: status.3,
            };
            status.1.apply_mut(&mut update_action);
            *status.2 = loc.row_id_word_id.clone();
        }
    }
}
