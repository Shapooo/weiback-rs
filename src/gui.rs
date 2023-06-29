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
    task_ongoing: bool,
    period: u32,
    executor: Executor,
    ratio: f32,
    task_status: Arc<RwLock<TaskStatus>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TaskType {
    Download,
    DownloadWithPic,
    DownloadExport,
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
            task_type: TaskType::Download,
            with_pic: true,
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
            ui.heading("WeiBack");
            ui.group(|ui| {
                ui.set_enabled(!self.task_ongoing);
                ui.horizontal(|ui| {
                    ui.label("Task type: ");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.task_type))
                        .show_ui(ui, |ui| {
                            for task_type in [
                                TaskType::Download,
                                TaskType::DownloadWithPic,
                                TaskType::DownloadExport,
                                TaskType::ExportFromLocal,
                            ] {
                                ui.selectable_value(
                                    &mut self.task_type,
                                    task_type,
                                    format!("{:?}", task_type),
                                );
                            }
                        });
                    if self.task_type != TaskType::Download {
                        self.with_pic = true;
                    }
                    let with_pic_cb = egui::Checkbox::new(&mut self.with_pic, "with pic");
                    ui.add_enabled(self.task_type == TaskType::Download, with_pic_cb);
                });
                ui.add(egui::Slider::new(&mut self.period, 1..=20).text("period"));
                ui.collapsing("advanced", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("from");
                        // ui.text_edit_singleline(&mut self.start_page);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.start_page).desired_width(50.0),
                        );
                        ui.label("-");
                        // ui.text_edit_singleline(&mut self.end_page);
                        ui.add(egui::TextEdit::singleline(&mut self.end_page).desired_width(50.0));
                    });
                });
                let start = self.start_page.parse::<u32>().unwrap_or(1);
                let end = self.end_page.parse::<u32>().unwrap_or(5);
                if ui.button("start").clicked() {
                    self.task_ongoing = true;
                    match self.task_type {
                        TaskType::Download => {
                            self.executor.download_meta(start..=end);
                        }
                        TaskType::DownloadWithPic => {
                            self.executor.download_with_pic(start..=end);
                        }
                        TaskType::DownloadExport => {
                            self.executor.export_from_net(start..=end);
                        }
                        TaskType::ExportFromLocal => {
                            self.executor.export_from_local(start..=end, false);
                        }
                    }
                }
            });
            ui.add(egui::ProgressBar::new(0.0).text(&self.message));
        });
    }
}
