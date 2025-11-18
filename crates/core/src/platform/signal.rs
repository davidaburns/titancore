use std::future::Future;
use std::sync::Arc;
use tokio::sync::Notify;

pub struct SignalWaiter {
    shutdown: Arc<Notify>,
}

impl SignalWaiter {
    pub fn new() -> Self {
        Self {
            shutdown: Arc::new(Notify::new()),
        }
    }

    pub async fn wait<F>(&self, task: F)
    where
        F: Future<Output = ()>,
    {
        let clone = self.shutdown.clone();
        tokio::spawn(async move {
            wait_for_signal().await;
            clone.notify_waiters();
        });

        tokio::pin!(task);
        tokio::select! {
            _ = self.shutdown.notified() => {}
            _ = &mut task => {}
        }
    }
}

async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        use tokio::signal;
        signal::ctrl_c().await.ok();
    }
}

impl Default for SignalWaiter {
    fn default() -> Self {
        Self::new()
    }
}
