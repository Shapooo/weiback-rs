use std::ops::RangeInclusive;

use eframe::egui::{self, DragValue};

use super::Tab;
use crate::core::{TaskRequest, TaskOptions};

pub struct BackupFavTab {
    web_total: u32,
    range: RangeInclusive<u32>,
    with_pic: bool,
    image_definition: u8,
}

impl Default for BackupFavTab {
    fn default() -> Self {
        Self {
            web_total: 0,
            range: 1..=0,
            with_pic: true,
            image_definition: 2,
        }
    }
}

impl BackupFavTab {
    pub fn set_web_total(&mut self, total: u32) {
        self.web_total = total;
        if self.range.end() == &0 {
            self.range = 1..=total;
        }
    }
}

impl Tab for BackupFavTab {
    fn name(&self) -> &str {
        "备份收藏"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Option<TaskRequest> {
        let mut task = None;
        ui.checkbox(&mut self.with_pic, "同时下载图片");
        ui.horizontal(|ui| {
            ui.label("图片清晰度：");
            ui.selectable_value(&mut self.image_definition, 0, "最低");
            ui.selectable_value(&mut self.image_definition, 1, "中等");
            ui.selectable_value(&mut self.image_definition, 2, "最高");
        });
        ui.collapsing("高级设置", |ui| {
            ui.horizontal(|ui| {
                ui.label("下载范围：");
                let (start, end) = self.range.clone().into_inner();
                let mut new_start = start;
                let mut new_end = end;
                ui.add(DragValue::new(&mut new_start).range(1..=new_end).speed(20));
                ui.label("-");
                ui.add(
                    DragValue::new(&mut new_end)
                        .range(new_start..=self.web_total)
                        .speed(20),
                );
                self.range = new_start..=new_end;
            });
        });

        ui.vertical_centered(|ui| {
            if ui.button("开始").clicked() {
                let options = TaskOptions::new()
                    .range(self.range.clone())
                    .with_pic(self.with_pic)
                    .pic_quality(self.image_definition.into());
                task = Some(TaskRequest::BackupFavorites(options));
            }
        });

        task
    }
}
