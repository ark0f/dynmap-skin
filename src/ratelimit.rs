use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{delay_for, Duration};

pub struct RateLimit {
    counter: AtomicU64,
    limit: u64,
}

impl RateLimit {
    pub fn new(limit: u64) -> Self {
        Self {
            counter: AtomicU64::default(),
            limit,
        }
    }

    pub async fn wait(&self) {
        let seconds = self.counter.fetch_add(self.limit, Ordering::AcqRel);
        delay_for(Duration::from_secs(seconds)).await;
        if seconds != 0 {
            self.counter.fetch_sub(self.limit, Ordering::AcqRel);
        }
    }
}
