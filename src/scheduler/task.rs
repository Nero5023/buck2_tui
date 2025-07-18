use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::hooks::Hooks;

pub type TaskId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStage {
    Pending,
    Dispatched,
    Hooked,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
}

pub type TaskOnSuccess =
    Box<dyn FnOnce(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

// Task that runs cmds
pub struct Task {
    pub id: TaskId,
    pub stage: TaskStage,
    pub priority: Priority,
    pub hooks: Arc<Hooks>,
    task_on_success: Option<TaskOnSuccess>,
    pub(crate) cmds: Vec<String>,
    pub(crate) current_dir: PathBuf,
    pub cancel_token: CancellationToken,
}

impl Task {
    pub fn new(
        priority: Priority,
        cmds: Vec<String>,
        current_dir: PathBuf,
        task_on_success: TaskOnSuccess,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            stage: TaskStage::Pending,
            priority,
            hooks: Arc::new(Hooks::new()),
            task_on_success: Some(task_on_success),
            cmds,
            current_dir,
            cancel_token: CancellationToken::new(),
        }
    }

    pub fn dispatch(&mut self) {
        self.stage = TaskStage::Dispatched;
    }

    pub fn hook(&mut self) {
        self.stage = TaskStage::Hooked;
    }

    // pub fn take_future(&mut self) -> Option<TaskOnSuccess> {
    //     self.future.take()
    // }

    pub(crate) fn take_task_on_success(&mut self) -> Option<TaskOnSuccess> {
        self.task_on_success.take()
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("stage", &self.stage)
            .field("priority", &self.priority)
            .field("has_task_on_success", &self.task_on_success.is_some())
            .finish()
    }
}
