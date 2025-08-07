use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::Task;
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct TaskManger {
    tasks: Arc<Mutex<HashMap<u64, Task>>>,
}

impl TaskManger {
    pub fn new() -> Self {
        let tasks = Arc::new(Mutex::new(HashMap::new()));
        Self { tasks }
    }

    pub fn new_task(&self, id: u64, task: Task) -> Result<()> {
        self.tasks
            .lock()
            .unwrap()
            .insert(id, task)
            .map_or(Ok(()), |_| {
                Err(Error::Other("Duplicate task id".to_string()))
            })
    }

    pub fn update_progress(
        &self,
        task_id: u64,
        progress_increment: u64,
        total_increment: u64,
    ) -> Result<(u64, u64)> {
        let mut total_new = 0;
        let mut progress_new = 0;
        self.tasks
            .lock()
            .map_err(|err| Error::Other(err.to_string()))?
            .entry(task_id)
            .and_modify(
                |Task {
                     total, progress, ..
                 }| {
                    *total += total_increment;
                    *progress += progress_increment;
                    total_new = *total;
                    progress_new = *progress;
                },
            );
        Ok((progress_new, total_new))
    }
}
