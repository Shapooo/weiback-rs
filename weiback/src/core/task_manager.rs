use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};

use log::{debug, error};

use super::Task;
use crate::error::{Error, Result};

#[derive(Clone, Default)]
pub struct TaskManger {
    tasks: Arc<Mutex<HashMap<u64, Task>>>,
}

impl TaskManger {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_task(&self, id: u64, task: Task) -> Result<()> {
        let mut tasks = self.tasks.lock().unwrap();
        match tasks.entry(id) {
            Entry::Occupied(_) => {
                error!("Duplicate task id: {id}");
                Err(Error::InconsistentTask("Duplicate task id".to_string()))
            }
            Entry::Vacant(v) => {
                v.insert(task);
                Ok(())
            }
        }
    }

    pub fn update_progress(
        &self,
        task_id: u64,
        progress_increment: u64,
        total_increment: u64,
    ) -> Result<(u64, u64)> {
        debug!(
            "Updating progress for task {task_id}: progress_increment={progress_increment}, total_increment={total_increment}"
        );
        let mut tasks = self.tasks.lock().map_err(|err| {
            error!("Failed to lock tasks mutex: {err}");
            err
        })?;

        match tasks.entry(task_id) {
            Entry::Occupied(mut o) => {
                let task = o.get_mut();
                task.total += total_increment;
                task.progress += progress_increment;
                let (progress_new, total_new) = (task.progress, task.total);
                debug!(
                    "Task {task_id} progress updated: progress={progress_new}, total={total_new}"
                );
                Ok((progress_new, total_new))
            }
            Entry::Vacant(_) => {
                error!("Task with id {task_id} not found for progress update");
                Err(Error::InconsistentTask(format!(
                    "Task with id {task_id} not found for progress update"
                )))
            }
        }
    }
}
