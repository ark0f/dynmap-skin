use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::time::{delay_for, Duration};

pub struct RateLimit {
    counter: Arc<AtomicU64>,
    limit: u64,
}

impl RateLimit {
    pub fn new(limit: u64) -> Self {
        Self {
            counter: Arc::default(),
            limit,
        }
    }

    pub async fn wait(&self) {
        let limit = self.limit;
        let counter = Arc::clone(&self.counter);
        let seconds = counter.fetch_add(limit, Ordering::AcqRel);

        delay_for(Duration::from_secs(seconds)).await;

        if seconds != 0 {
            counter.fetch_sub(limit, Ordering::AcqRel);
        }
    }
}
