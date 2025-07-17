use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

use super::hooks::Hooks;

pub type TaskId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStage {
    Pending,
    Dispatched,
    Hooked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    User,
    Preload,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
}

pub type TaskFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub struct Task {
    pub id: TaskId,
    pub stage: TaskStage,
    pub task_type: TaskType,
    pub priority: Priority,
    pub hooks: Arc<Hooks>,
    pub future: Option<TaskFuture>,
}

impl Task {
    pub fn new(
        task_type: TaskType,
        priority: Priority,
        future: TaskFuture,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            stage: TaskStage::Pending,
            task_type,
            priority,
            hooks: Arc::new(Hooks::new()),
            future: Some(future),
        }
    }

    pub fn user(priority: Priority, future: TaskFuture) -> Self {
        Self::new(TaskType::User, priority, future)
    }

    pub fn preload(priority: Priority, future: TaskFuture) -> Self {
        Self::new(TaskType::Preload, priority, future)
    }

    pub fn dispatch(&mut self) {
        self.stage = TaskStage::Dispatched;
    }

    pub fn hook(&mut self) {
        self.stage = TaskStage::Hooked;
    }

    pub fn take_future(&mut self) -> Option<TaskFuture> {
        self.future.take()
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("stage", &self.stage)
            .field("task_type", &self.task_type)
            .field("priority", &self.priority)
            .field("has_future", &self.future.is_some())
            .finish()
    }
}