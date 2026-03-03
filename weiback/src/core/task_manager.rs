//! This module provides the infrastructure for tracking the lifecycle of asynchronous tasks.
//!
//! The [`TaskManager`] allows the application to:
//! - Monitor the progress of a currently running task.
//! - Retrieve error messages if a task or its sub-tasks fail.
//! - Ensure that only one long-running task is active at a time.

use serde::Serialize;
use std::sync::{Arc, Mutex};

use crate::error::{Error, Result};

/// The general category of an asynchronous task.
#[derive(Debug, Clone, Serialize)]
pub enum TaskType {
    /// Backup posts from a specific user.
    BackupUser,
    /// Backup favorited posts.
    BackupFavorites,
    /// Unfavorite posts that are already in local storage but still favorited on Weibo.
    UnfavoritePosts,
    /// Export posts from local storage to external formats.
    Export,
    /// Clean up redundant or low-resolution images.
    CleanupPictures,
    /// Clean up invalid or outdated avatars.
    CleanupAvatars,
}

/// The current execution state of a task.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TaskStatus {
    /// The task is currently running.
    InProgress,
    /// The task has finished successfully.
    Completed,
    /// The task has stopped due to a fatal error.
    Failed,
}

/// Represents a single unit of work being performed by the application.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    /// The unique task ID.
    pub id: u64,
    /// The general category of the task.
    pub task_type: TaskType,
    /// A human-readable summary of the task.
    pub description: String,
    /// The current state of the task (InProgress, Completed, Failed).
    pub status: TaskStatus,
    /// Current completion progress (e.g., number of pages fetched).
    pub progress: u64,
    /// The total estimated progress for completion.
    pub total: u64,
    /// An optional error message if the task failed.
    pub error: Option<String>,
}

/// Types of errors that can occur within a sub-task (e.g., individual file download).
#[derive(Debug, Clone, Serialize)]
pub enum SubTaskErrorType {
    /// Failed to download a specific media file. Contains the URL.
    DownloadMedia(String),
}

/// A non-fatal error record for a specific sub-operation within a larger task.
#[derive(Debug, Clone, Serialize)]
pub struct SubTaskError {
    /// The category of the error.
    pub error_type: SubTaskErrorType,
    /// A detailed error message.
    pub message: String,
}

/// A thread-safe manager for monitoring the execution state of application tasks.
///
/// `TaskManager` ensures that long-running operations can be monitored from the
/// UI and prevents multiple conflicting tasks from running simultaneously.
#[derive(Debug, Clone, Default)]
pub struct TaskManager {
    current_task: Arc<Mutex<Option<Task>>>,
    sub_task_errors: Arc<Mutex<Vec<SubTaskError>>>,
}

impl TaskManager {
    /// Creates a new, empty `TaskManager`.
    pub fn new() -> Self {
        Self {
            current_task: Arc::new(Mutex::new(None)),
            sub_task_errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registers and starts a new task.
    ///
    /// # Arguments
    /// * `id` - A unique identifier for the task.
    /// * `task_type` - The category of the task.
    /// * `description` - A human-readable description of what the task does.
    /// * `total` - The initial estimate of total work units (can be updated later).
    ///
    /// # Errors
    /// Returns `Error::InconsistentTask` if another task is already `InProgress`.
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

    /// Updates the progress and total units of the currently active task.
    ///
    /// # Arguments
    /// * `progress_increment` - Value to add to the current progress.
    /// * `total_increment` - Value to add to the current total units.
    ///
    /// # Errors
    /// Returns `Error::InconsistentTask` if no task is currently `InProgress`.
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

    /// Marks the current task as `Completed`.
    ///
    /// # Errors
    /// Returns `Error::InconsistentTask` if no task is currently active.
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

    /// Marks the current task as `Failed` and records an error message.
    ///
    /// # Arguments
    /// * `error` - The error message explaining the failure.
    ///
    /// # Errors
    /// Returns `Error::InconsistentTask` if no task is currently active.
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

    /// Records a non-fatal sub-task error.
    ///
    /// These errors do not stop the main task but are collected for reporting.
    ///
    /// # Arguments
    /// * `error` - The `SubTaskError` to record.
    pub fn add_sub_task_error(&self, error: SubTaskError) -> Result<()> {
        self.sub_task_errors.lock()?.push(error);
        Ok(())
    }

    /// Retrieves all recorded sub-task errors and clears the internal list.
    ///
    /// # Returns
    /// A `Result` containing a `Vec` of `SubTaskError`s.
    pub fn get_and_clear_sub_task_errors(&self) -> Result<Vec<SubTaskError>> {
        let mut errors = self.sub_task_errors.lock()?;
        let ret = errors.drain(..).collect();
        Ok(ret)
    }

    /// Returns a clone of the currently registered task, if any.
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
