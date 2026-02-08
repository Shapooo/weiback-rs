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

#[derive(Debug, Clone, Serialize)]
pub enum SubTaskErrorType {
    DownloadMedia(String), // URL
}

#[derive(Debug, Clone, Serialize)]
pub struct SubTaskError {
    pub error_type: SubTaskErrorType,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct TaskManager {
    current_task: Arc<Mutex<Option<Task>>>,
    sub_task_errors: Arc<Mutex<Vec<SubTaskError>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            current_task: Arc::new(Mutex::new(None)),
            sub_task_errors: Arc::new(Mutex::new(Vec::new())),
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
        if let Some(existing_task) = task_guard.as_ref()
            && existing_task.status == TaskStatus::InProgress
        {
            return Err(Error::InconsistentTask(
                "Another task is already in progress.".to_string(),
            ));
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

    pub fn add_sub_task_error(&self, error: SubTaskError) -> Result<()> {
        self.sub_task_errors.lock()?.push(error);
        Ok(())
    }

    pub fn get_and_clear_sub_task_errors(&self) -> Result<Vec<SubTaskError>> {
        let mut errors = self.sub_task_errors.lock()?;
        let ret = errors.drain(..).collect();
        Ok(ret)
    }

    pub fn get_current(&self) -> Result<Option<Task>> {
        Ok(self.current_task.lock()?.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_new_task() {
        let manager = TaskManager::new();
        assert!(manager.get_current().unwrap().is_none());

        manager
            .start_task(1, TaskType::BackupUser, "Test task".into(), 10)
            .unwrap();

        let task = manager.get_current().unwrap().unwrap();
        assert_eq!(task.id, 1);
        assert_eq!(task.description, "Test task");
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.progress, 0);
        assert_eq!(task.total, 10);
        assert!(task.error.is_none());
    }

    #[test]
    fn test_prevent_starting_task_when_in_progress() {
        let manager = TaskManager::new();
        manager
            .start_task(1, TaskType::BackupUser, "First task".into(), 10)
            .unwrap();
        let result = manager.start_task(2, TaskType::BackupFavorites, "Second task".into(), 5);

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InconsistentTask(msg) => {
                assert!(msg.contains("Another task is already in progress."));
            }
            _ => panic!("Expected InconsistentTask error"),
        }
    }

    #[test]
    fn test_update_progress() {
        let manager = TaskManager::new();
        manager
            .start_task(1, TaskType::BackupUser, "Test task".into(), 10)
            .unwrap();

        manager.update_progress(5, 5).unwrap();
        let task = manager.get_current().unwrap().unwrap();
        assert_eq!(task.progress, 5);
        assert_eq!(task.total, 15);

        manager.update_progress(1, 0).unwrap();
        let task = manager.get_current().unwrap().unwrap();
        assert_eq!(task.progress, 6);
        assert_eq!(task.total, 15);
    }

    #[test]
    fn test_finish_task() {
        let manager = TaskManager::new();
        manager
            .start_task(1, TaskType::BackupUser, "Test task".into(), 10)
            .unwrap();
        manager.finish().unwrap();
        let task = manager.get_current().unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn test_fail_task() {
        let manager = TaskManager::new();
        manager
            .start_task(1, TaskType::BackupUser, "Test task".into(), 10)
            .unwrap();
        let error_msg = "Something went wrong".to_string();
        manager.fail(error_msg.clone()).unwrap();
        let task = manager.get_current().unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.error, Some(error_msg));
    }

    #[test]
    fn test_sub_task_error_handling() {
        let manager = TaskManager::new();
        assert!(manager.get_and_clear_sub_task_errors().unwrap().is_empty());

        let error1 = SubTaskError {
            error_type: SubTaskErrorType::DownloadMedia("url1".into()),
            message: "404 Not Found".into(),
        };
        let error2 = SubTaskError {
            error_type: SubTaskErrorType::DownloadMedia("url2".into()),
            message: "Timeout".into(),
        };

        manager.add_sub_task_error(error1.clone()).unwrap();
        manager.add_sub_task_error(error2.clone()).unwrap();

        let errors = manager.get_and_clear_sub_task_errors().unwrap();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].message, "404 Not Found");
        assert_eq!(errors[1].message, "Timeout");

        // Verify that the error list is cleared
        assert!(manager.get_and_clear_sub_task_errors().unwrap().is_empty());
    }

    #[test]
    fn test_start_new_task_after_completion() {
        let manager = TaskManager::new();
        manager
            .start_task(1, TaskType::BackupUser, "First task".into(), 10)
            .unwrap();
        manager.finish().unwrap();

        // Should be able to start a new task
        let result = manager.start_task(2, TaskType::BackupFavorites, "Second task".into(), 5);
        assert!(result.is_ok());
        let task = manager.get_current().unwrap().unwrap();
        assert_eq!(task.id, 2);
        assert_eq!(task.status, TaskStatus::InProgress);
    }
}
