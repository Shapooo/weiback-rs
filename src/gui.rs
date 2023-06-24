use eframe::{
    egui::{self, vec2},
    NativeOptions,
};

use crate::config::Config;
use crate::executor::Executor;

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
        eframe::run_native("weiback", self.options, Box::new(|_cc| Box::new(self.gui))).unwrap();
    }
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
        Self {
            start_page: "".into(),
            end_page: "".into(),
            message: "".into(),
            task_type: TaskType::Download,
            with_pic: true,
            task_ongoing: false,
            period: 10,
            executor: Executor::new(config),
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("WeiBack");
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
                // ui.checkbox(&mut self.with_pic, "with pic");
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
                    ui.add(egui::TextEdit::singleline(&mut self.start_page).desired_width(50.0));
                    ui.label("-");
                    // ui.text_edit_singleline(&mut self.end_page);
                    ui.add(egui::TextEdit::singleline(&mut self.end_page).desired_width(50.0));
                });
            });
            let start = self.start_page.parse::<u32>().unwrap_or(1);
            let end = self.end_page.parse::<u32>().unwrap_or(2);
            if ui.button("start").clicked() {
                // self.task_ongoing = true;
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
    }
}
