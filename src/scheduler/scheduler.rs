use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_priority_channel::{Receiver, Sender, unbounded};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::task::{Task, TaskId, Priority};

#[derive(Debug)]
pub struct Ongoing {
    pub all: HashMap<TaskId, Task>,
    pub micro_handles: HashMap<TaskId, JoinHandle<()>>,
    pub macro_handles: HashMap<TaskId, JoinHandle<()>>,
}

impl Ongoing {
    pub fn new() -> Self {
        Self {
            all: HashMap::new(),
            micro_handles: HashMap::new(),
            macro_handles: HashMap::new(),
        }
    }

    pub fn remove(&mut self, id: &TaskId) -> Option<Task> {
        self.micro_handles.remove(id).map(|h| h.abort());
        self.macro_handles.remove(id).map(|h| h.abort());
        self.all.remove(id)
    }
}

pub struct Scheduler {
    micro_tx: Sender<Task, Priority>,
    macro_tx: Sender<Task, Priority>,
    ongoing: Arc<Mutex<Ongoing>>,
    cancel_token: CancellationToken,
}

impl Scheduler {
    pub fn new() -> Self {
        let (micro_tx, micro_rx) = unbounded();
        let (macro_tx, macro_rx) = unbounded();
        let ongoing = Arc::new(Mutex::new(Ongoing::new()));
        let cancel_token = CancellationToken::new();

        let scheduler = Self {
            micro_tx,
            macro_tx,
            ongoing: ongoing.clone(),
            cancel_token: cancel_token.clone(),
        };

        scheduler.spawn_workers(micro_rx, macro_rx, ongoing, cancel_token);
        scheduler
    }

    fn spawn_workers(
        &self,
        micro_rx: Receiver<Task, Priority>,
        macro_rx: Receiver<Task, Priority>,
        ongoing: Arc<Mutex<Ongoing>>,
        cancel_token: CancellationToken,
    ) {
        let ongoing_micro = ongoing.clone();
        let cancel_micro = cancel_token.clone();
        tokio::spawn(async move {
            Self::worker_loop(micro_rx, ongoing_micro, cancel_micro, true).await;
        });

        let ongoing_macro = ongoing.clone();
        let cancel_macro = cancel_token.clone();
        tokio::spawn(async move {
            Self::worker_loop(macro_rx, ongoing_macro, cancel_macro, false).await;
        });
    }

    async fn worker_loop(
        mut rx: Receiver<Task, Priority>,
        ongoing: Arc<Mutex<Ongoing>>,
        cancel_token: CancellationToken,
        is_micro: bool,
    ) {
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    break;
                }
                task = rx.recv() => {
                    if let Ok((mut task, _priority)) = task {
                        task.dispatch();
                        let task_id = task.id;
                        
                        if let Some(future) = task.take_future() {
                            let ongoing_clone = ongoing.clone();
                            let handle = tokio::spawn(async move {
                                future.await;
                                
                                let hooks = {
                                    let mut ongoing = ongoing_clone.lock().await;
                                    if let Some(mut task) = ongoing.all.remove(&task_id) {
                                        task.hook();
                                        Some(task.hooks.clone())
                                    } else {
                                        None
                                    }
                                };
                                
                                if let Some(hooks) = hooks {
                                    hooks.run_all().await;
                                }
                            });

                            let mut ongoing = ongoing.lock().await;
                            ongoing.all.insert(task_id, task);
                            
                            if is_micro {
                                ongoing.micro_handles.insert(task_id, handle);
                            } else {
                                ongoing.macro_handles.insert(task_id, handle);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn dispatch_micro(&self, task: Task) {
        let priority = task.priority.clone();
        let tx = self.micro_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(task, priority).await;
        });
    }

    pub fn dispatch_macro(&self, task: Task) {
        let priority = task.priority.clone();
        let tx = self.macro_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(task, priority).await;
        });
    }

    pub fn cancel(&self, id: TaskId) -> bool {
        let ongoing = self.ongoing.clone();
        tokio::spawn(async move {
            let mut ongoing = ongoing.lock().await;
            if let Some(task) = ongoing.remove(&id) {
                let hooks = task.hooks.clone();
                drop(ongoing);
                
                hooks.run_all().await;
            }
        });
        
        true // Return true optimistically; actual cancellation happens async
    }

    pub fn cancel_all(&self) {
        let ongoing = self.ongoing.clone();
        tokio::spawn(async move {
            let mut ongoing = ongoing.lock().await;
            let tasks: Vec<_> = ongoing.all.drain().collect();
            
            for handle in ongoing.micro_handles.drain() {
                handle.1.abort();
            }
            for handle in ongoing.macro_handles.drain() {
                handle.1.abort();
            }
            
            drop(ongoing);

            for (_, task) in tasks {
                let hooks = task.hooks.clone();
                tokio::spawn(async move {
                    hooks.run_all().await;
                });
            }
        });
    }

    pub async fn get_ongoing_tasks(&self) -> Vec<TaskId> {
        let ongoing = self.ongoing.lock().await;
        ongoing.all.keys().cloned().collect()
    }

    pub async fn has_task(&self, id: &TaskId) -> bool {
        let ongoing = self.ongoing.lock().await;
        ongoing.all.contains_key(id)
    }

    pub fn shutdown(&self) {
        self.cancel_all();
        self.cancel_token.cancel();
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.shutdown();
    }
}