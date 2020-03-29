use std::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{delay_for, Duration};

pub struct Ratelimit((Sender<()>, Receiver<()>));

impl Ratelimit {
    pub async fn wait(&self) {
        let (tx, rx) = &self.0;
        if rx.try_recv().is_err() {
            delay_for(Duration::from_secs(30)).await;
            let _ = tx.send(());
        }
    }
}

impl Default for Ratelimit {
    fn default() -> Self {
        Self(channel())
    }
}
