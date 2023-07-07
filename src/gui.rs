use std::sync::{Arc, RwLock};

use eframe::{
    egui::{self, vec2},
    NativeOptions,
};
use log::info;

use crate::config::Config;
use crate::executor::Executor;
use crate::message::TaskStatus;

pub struct WbApp {
    options: NativeOptions,
    gui: Gui,
}

impl WbApp {
    pub fn new(config: Config) -> Self {
        Self {
            options: NativeOptions {
                initial_window_size: Some(vec2(300.0, 300.0)),
                ..Default::default()
            },
            gui: Gui::new(config),
        }
    }
    pub fn run(self) {
        info!("starting gui...");
        eframe::run_native(
            "weiback",
            self.options,
            Box::new(|cc| {
                set_font(cc);
                Box::new(self.gui)
            }),
        )
        .unwrap()
    }
}

fn set_font(cc: &eframe::CreationContext) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "source".into(),
        egui::FontData::from_static(include_bytes!("../res/fonts/SourceHanSansCN-Medium.otf")),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("source".into());
    cc.egui_ctx.set_fonts(fonts);
}

struct Gui {
    start_page: String,
    end_page: String,
    message: String,
    task_type: TaskType,
    with_pic: bool,
    export: bool,
    task_ongoing: bool,
    period: u32,
    executor: Executor,
    ratio: f32,
    task_status: Arc<RwLock<TaskStatus>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TaskType {
    DownloadPosts,
    ExportFromLocal,
}

impl Gui {
    fn new(config: Config) -> Self {
        let task_status: Arc<RwLock<TaskStatus>> = Arc::default();
        let executor = Executor::new(config, task_status.clone());
        Self {
            start_page: Default::default(),
            end_page: Default::default(),
            message: Default::default(),
            task_type: TaskType::DownloadPosts,
            with_pic: true,
            export: true,
            task_ongoing: false,
            period: 10,
            ratio: 0.0,
            executor,
            task_status,
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let task_status: Option<TaskStatus> = self
            .task_status
            .try_read()
            .ok()
            .map(|task_status| task_status.clone());
        if let Some(task_status) = task_status {
            match &task_status {
                TaskStatus::InProgress(ratio, msg) => {
                    self.ratio = *ratio;
                    self.message = msg.to_owned()
                }
                TaskStatus::Finished => {
                    self.task_ongoing = false;
                }
                TaskStatus::Error(msg) => {
                    self.message = msg.to_owned();
                }
                TaskStatus::Info(msg) => {
                    self.message = msg.to_owned();
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| ui.heading("WeiBack"));
            ui.group(|ui| {
                ui.set_enabled(!self.task_ongoing);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.task_type, TaskType::DownloadPosts, "从网络下载");
                    ui.selectable_value(
                        &mut self.task_type,
                        TaskType::ExportFromLocal,
                        "从本地导出",
                    );
                });
                if self.task_type == TaskType::DownloadPosts {
                    let old_with_pic = self.with_pic;
                    ui.checkbox(&mut self.with_pic, "附带图片");
                    ui.checkbox(&mut self.export, "导出");
                    if !self.with_pic && self.export {
                        if old_with_pic {
                            self.export = false;
                        } else {
                            self.with_pic = true;
                        }
                    }
                }

                ui.collapsing("高级设置", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("下载范围：");
                        // ui.text_edit_singleline(&mut self.start_page);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.start_page).desired_width(50.0),
                        )
                        .on_hover_text("testtest");
                        ui.label("-");
                        // ui.text_edit_singleline(&mut self.end_page);
                        ui.add(egui::TextEdit::singleline(&mut self.end_page).desired_width(50.0))
                            .on_hover_text("testtest");
                    });
                    ui.add(egui::Slider::new(&mut self.period, 1..=20).text("每页"));
                });
            });
            let start = self.start_page.parse::<u32>().unwrap_or(1);
            let end = self.end_page.parse::<u32>().unwrap_or(u32::MAX);
            ui.vertical_centered(|ui| {
                ui.set_enabled(!self.task_ongoing);
                if ui.button("开始").clicked() {
                    self.task_ongoing = true;
                    match self.task_type {
                        TaskType::DownloadPosts => {
                            self.executor.download_posts(start..=end, self.with_pic);
                        }
                        TaskType::ExportFromLocal => {
                            self.executor.export_from_local(start..=end, false);
                        }
                    }
                }
            });
            ui.add(egui::ProgressBar::new(self.ratio));
            ui.vertical_centered(|ui| {
                ui.label(&self.message);
            });
        });
    }
}
