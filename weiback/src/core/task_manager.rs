use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize)]
pub enum TaskType {
    BackupUser,
    BackupFavorites,
    UnfavoritePosts,
    Export,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TaskStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: u64,
    pub task_type: TaskType,
    pub description: String,
    pub status: TaskStatus,
    pub progress: u64,
    pub total: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskManager {
    current_task: Arc<Mutex<Option<Task>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            current_task: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start_task(
        &self,
        id: u64,
        task_type: TaskType,
        description: String,
        total: u64,
    ) -> Result<()> {
        let mut task_guard = self.current_task.lock()?;
        if let Some(existing_task) = task_guard.as_ref() {
            if existing_task.status == TaskStatus::InProgress {
                return Err(Error::InconsistentTask(
                    "Another task is already in progress.".to_string(),
                ));
            }
        }

        let new_task = Task {
            id,
            task_type,
            description,
            status: TaskStatus::InProgress,
            progress: 0,
            total,
            error: None,
        };
        *task_guard = Some(new_task);
        Ok(())
    }

    pub fn update_progress(&self, progress_increment: u64, total_increment: u64) -> Result<()> {
        let mut task_guard = self.current_task.lock()?;
        if let Some(task) = task_guard.as_mut() {
            if task.status == TaskStatus::InProgress {
                task.progress += progress_increment;
                task.total += total_increment;
            }
            Ok(())
        } else {
            Err(Error::InconsistentTask(
                "Cannot update progress: no task is in progress.".to_string(),
            ))
        }
    }

    pub fn finish(&self) -> Result<()> {
        let mut task_guard = self.current_task.lock()?;
        if let Some(task) = task_guard.as_mut() {
            task.status = TaskStatus::Completed;
            Ok(())
        } else {
            Err(Error::InconsistentTask(
                "Cannot finish task: no task is in progress.".to_string(),
            ))
        }
    }

    pub fn fail(&self, error: String) -> Result<()> {
        let mut task_guard = self.current_task.lock()?;
        if let Some(task) = task_guard.as_mut() {
            task.status = TaskStatus::Failed;
            task.error = Some(error);
            Ok(())
        } else {
            Err(Error::InconsistentTask(
                "Cannot fail task: no task is in progress.".to_string(),
            ))
        }
    }

    pub fn get_current(&self) -> Result<Option<Task>> {
        Ok(self.current_task.lock()?.clone())
    }
}
