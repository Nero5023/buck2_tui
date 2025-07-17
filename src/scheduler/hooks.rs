use std::future::Future;
use std::pin::Pin;
use tokio::sync::Mutex;

pub type SyncHook = Box<dyn FnOnce() + Send + 'static>;
pub type AsyncHook = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub enum Hook {
    Sync(SyncHook),
    Async(AsyncHook),
}

pub struct Hooks {
    inner: Mutex<Vec<Hook>>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
        }
    }

    pub async fn add_sync<F>(&self, hook: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut hooks = self.inner.lock().await;
        hooks.push(Hook::Sync(Box::new(hook)));
    }

    pub async fn add_async<F>(&self, hook: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut hooks = self.inner.lock().await;
        hooks.push(Hook::Async(Box::pin(hook)));
    }

    pub async fn run_all(&self) {
        let mut hooks_guard = self.inner.lock().await;
        let hooks = std::mem::take(&mut *hooks_guard);
        drop(hooks_guard);

        for hook in hooks {
            match hook {
                Hook::Sync(f) => f(),
                Hook::Async(f) => f.await,
            }
        }
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}
