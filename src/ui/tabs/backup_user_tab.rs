use eframe::egui;

use super::Tab;
use crate::core::{Task, TaskOptions};

pub struct BackupUserTab {
    uid_str: String,
    with_pic: bool,
    image_definition: u8,
}

impl Default for BackupUserTab {
    fn default() -> Self {
        Self {
            uid_str: Default::default(),
            with_pic: true,
            image_definition: 2,
        }
    }
}

impl Tab for BackupUserTab {
    fn name(&self) -> &str {
        "备份用户"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Option<Task> {
        let mut task = None;

        ui.checkbox(&mut self.with_pic, "同时下载图片");
        ui.horizontal(|ui| {
            ui.label("图片清晰度：");
            ui.selectable_value(&mut self.image_definition, 0, "最低");
            ui.selectable_value(&mut self.image_definition, 1, "中等");
            ui.selectable_value(&mut self.image_definition, 2, "最高");
        });
        ui.horizontal(|ui| {
            ui.label("下载用户ID，默认自己：");
            ui.text_edit_singleline(&mut self.uid_str);
        });

        ui.vertical_centered(|ui| {
            if ui.button("开始").clicked() {
                let uid = if self.uid_str.is_empty() {
                    0
                } else {
                    self.uid_str.parse().unwrap()
                };
                let options = TaskOptions::new()
                    .with_user(uid)
                    .with_pic(self.with_pic)
                    .pic_quality(self.image_definition.into());
                task = Some(Task::BackupUser(options));
            }
        });

        task
    }
}
