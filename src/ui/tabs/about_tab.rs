use eframe::egui;

use super::{Tab, TaskRequest};

#[derive(Default)]
pub struct AboutTab;

impl Tab for AboutTab {
    fn name(&self) -> &str {
        "关于"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Option<TaskRequest> {
        use egui::special_emojis;
        ui.heading("WeiBack-rs");
        ui.label("WeiBack-rs 是一个开源微博备份工具。");
        ui.label(format!(
            "SUPPORTED PLATFORM: {} Linux/{} Windows",
            special_emojis::OS_LINUX,
            special_emojis::OS_WINDOWS
        ));
        ui.label(format!(
            "You can build by yourself on {} macOS",
            special_emojis::OS_APPLE
        ));
        ui.label("AUTHER: Shapooo");
        ui.label("LICENSE: MIT");
        ui.hyperlink_to(
            format!("{} REPOSITORY LINK", special_emojis::GITHUB),
            "https://github.com/shapooo/weiback-rs",
        );
        None
    }
}
