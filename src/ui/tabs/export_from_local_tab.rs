use std::ops::RangeInclusive;

use eframe::egui::{self, DragValue, Slider};

use super::{Tab, Task};
use crate::exporter::ExportOptions;

pub struct ExportFromLocalTab {
    db_total: u32,
    range: RangeInclusive<u32>,
    reverse: bool,
    period: u32,
    image_definition: u8,
}

impl Default for ExportFromLocalTab {
    fn default() -> Self {
        Self {
            db_total: 0,
            range: 1..=0,
            reverse: true,
            period: 50,
            image_definition: 2,
        }
    }
}

impl ExportFromLocalTab {
    pub fn set_db_total(&mut self, total: u32) {
        self.db_total = total;
        if self.range.end() == &0 {
            self.range = 1..=total;
        }
    }
}

impl Tab for ExportFromLocalTab {
    fn name(&self) -> &str {
        "本地导出"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Option<Task> {
        let mut task = None;

        ui.checkbox(&mut self.reverse, "按时间逆序")
            .on_hover_text("时间逆序即最上方的微博为最新的微博");
        ui.horizontal(|ui| {
            ui.label("图片清晰度：");
            ui.selectable_value(&mut self.image_definition, 0, "最低");
            ui.selectable_value(&mut self.image_definition, 1, "中等");
            ui.selectable_value(&mut self.image_definition, 2, "最高");
        });
        ui.collapsing("高级设置", |ui| {
            ui.horizontal(|ui| {
                ui.label("导出范围：");
                let (start, end) = self.range.clone().into_inner();
                let mut new_start = start;
                let mut new_end = end;
                ui.add(DragValue::new(&mut new_start).range(1..=new_end).speed(20));
                ui.label("-");
                ui.add(
                    DragValue::new(&mut new_end)
                        .range(new_start..=self.db_total)
                        .speed(self.period),
                );
                self.range = new_start..=new_end;
            });
            ui.add(Slider::new(&mut self.period, 10..=200).text("每页"))
                .on_hover_text("导出时默认50条博文分割为一个html文件");
        });

        if ui.button("对本地微博取消收藏").clicked() {
            task = Some(Task::UnfavoritePosts);
        }

        ui.vertical_centered(|ui| {
            if ui.button("开始").clicked() {
                let options = ExportOptions::new()
                    .reverse(self.reverse)
                    .range(self.range.clone())
                    .pic_quality(self.image_definition.into());
                task = Some(Task::ExportFromLocal(options));
            }
        });

        task
    }
}
