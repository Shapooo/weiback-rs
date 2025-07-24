pub mod about_tab;
pub mod backup_fav_tab;
pub mod backup_user_tab;
pub mod export_from_local_tab;

use crate::core::TaskRequest;

/// The trait that all tab must implement
pub trait Tab {
    /// Get the name of the tab
    fn name(&self) -> &str;

    /// Show the UI of the tab
    /// return a task to be executed
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> Option<TaskRequest>;
}
