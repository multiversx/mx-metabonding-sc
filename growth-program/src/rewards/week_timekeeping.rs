use crate::{Timestamp, WEEK_IN_SECONDS};

multiversx_sc::imports!();

pub const FIRST_WEEK: Week = 1;
pub const MONDAY_19_02_2024_GMT_TIMESTAMP: u64 = 1_708_300_800;
static INVALID_WEEK_ERR_MSG: &[u8] = b"Week 0 is not a valid week";

pub type Week = usize;
pub type Epoch = u64;

#[multiversx_sc::module]
pub trait WeekTimekeepingModule {
    #[view(getFirstWeekStartTimestamp)]
    fn get_first_week_start_timestamp(&self) -> Timestamp {
        MONDAY_19_02_2024_GMT_TIMESTAMP
    }

    /// Week starts from 1
    #[view(getCurrentWeek)]
    fn get_current_week(&self) -> Week {
        let current_timestamp = self.blockchain().get_block_timestamp();
        self.get_week_for_timestamp(current_timestamp)
    }

    fn get_week_for_timestamp(&self, timestamp: Timestamp) -> Week {
        let first_week_start_timestamp = MONDAY_19_02_2024_GMT_TIMESTAMP;
        require!(
            timestamp >= first_week_start_timestamp,
            INVALID_WEEK_ERR_MSG
        );

        unsafe {
            // will never overflow usize
            let zero_based_week: Week = ((timestamp - first_week_start_timestamp)
                / WEEK_IN_SECONDS)
                .try_into()
                .unwrap_unchecked();

            zero_based_week + 1
        }
    }
}
