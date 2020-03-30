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
            counter: Arc::new(AtomicU64::new(0)),
            limit,
        }
    }

    pub async fn wait(&self) {
        let limit = self.limit;
        let counter = Arc::clone(&self.counter);
        let seconds = counter.fetch_add(limit, Ordering::SeqCst);
        delay_for(Duration::from_secs(seconds)).await;
        actix_rt::spawn(async move {
            delay_for(Duration::from_secs(limit)).await;
            counter.fetch_sub(limit, Ordering::SeqCst);
        })
    }
}
