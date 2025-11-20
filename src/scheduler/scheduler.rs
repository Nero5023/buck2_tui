use async_priority_channel::Receiver;
use async_priority_channel::Sender;
use async_priority_channel::unbounded;
use futures::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::task::Priority;
use super::task::Task;
use super::task::TaskId;

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
        rx: Receiver<Task, Priority>,
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
                    if let Ok((task, _priority)) = task {
                        // TODO: right now we just ignore is_micro
                        Self::handle_task(task, ongoing.clone()).await.unwrap();

                    }
                }
            }
        }
    }

    async fn handle_task(
        mut task: Task,
        // is_micro: bool,
        ongoing: Arc<Mutex<Ongoing>>,
    ) -> anyhow::Result<()> {
        if task.is_cancelled() {
            return Ok(());
        }

        task.dispatch();

        let task_id = task.id;

        if task.cmds.is_empty() {
            return Ok(());
        }

        let (program, args) = task.cmds.split_first().unwrap();
        let mut child = tokio::process::Command::new(program)
            .args(args)
            .current_dir(task.current_dir.clone())
            .stdin(std::process::Stdio::null()) // Don't inherit stdin
            .stdout(std::process::Stdio::piped()) // Capture stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr
            .spawn()?;

        let task_on_success = task.take_task_on_success();
        let cancel_token = task.cancel_token.clone();
        let hooks = task.hooks.clone();

        let _: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let (stdout, _stderr, status) = tokio::select! {
                result = async {
                    let (stdout_result, stderr_result, exit_status) = tokio::join!(
                        async {
                            let mut stdout_reader = BufReader::new(stdout);
                            let mut content = String::new();
                            stdout_reader.read_to_string(&mut content).await?;
                            Ok::<String, std::io::Error>(content)
                        },
                        async {
                            let mut stderr_reader = BufReader::new(stderr);
                            let mut content = String::new();
                            stderr_reader.read_to_string(&mut content).await?;
                            Ok::<String, std::io::Error>(content)
                        },
                        child.wait()
                    );

                    let stdout_content = stdout_result?;
                    let stderr_content = stderr_result?;
                    let exit_status = exit_status?;
                        Ok::<(String, String, i32), anyhow::Error>(
                        (stdout_content, stderr_content, exit_status.code().unwrap_or(-1))
                    )
                } => {
                    result?
                }
                _ = cancel_token.cancelled() => {
                    child.kill().await?;
                    child.wait().await?;
                    return Ok(());
                }
            };

            if status == 0 {
                if let Some(task_on_success) = task_on_success {
                    task_on_success(stdout).await;
                    hooks.run_all().await;
                }
            }

            Ok(())
        });

        let mut ongoing = ongoing.lock().await;
        ongoing.all.insert(task_id, task);

        Ok(())
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
                // Cancel the task's cancellation token to stop the work
                task.cancel();

                // Get the cleanup hooks before dropping the task
                let hooks = task.hooks.clone();
                drop(ongoing);

                // Run cleanup hooks
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

            // Cancel all tasks
            for (_, task) in &tasks {
                task.cancel();
            }

            // Abort all task handles
            for handle in ongoing.micro_handles.drain() {
                handle.1.abort();
            }
            for handle in ongoing.macro_handles.drain() {
                handle.1.abort();
            }

            drop(ongoing);

            // Run cleanup hooks for all tasks
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

async fn run_cancellable_command(
    token: CancellationToken,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = tokio::process::Command::new("your_command")
        .arg("arg1")
        .spawn()?;

    tokio::select! {
        // 等待进程完成
        result = child.wait() => {
            match result {
                Ok(status) => println!("Command completed with status: {}", status),
                Err(e) => println!("Command failed: {}", e),
            }
        }
        // 等待取消信号
        _ = token.cancelled() => {
            println!("Command was cancelled, killing process");
            child.kill().await?;
            child.wait().await?; // 等待进程清理
        }
    }

    Ok(())
}
