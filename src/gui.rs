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
    tab_type: TabType,
    with_pic: bool,
    task_ongoing: bool,
    period: u32,
    executor: Executor,
    ratio: f32,
    task_status: Arc<RwLock<TaskStatus>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TabType {
    DownloadPosts,
    ExportFromLocal,
    About,
}

impl Gui {
    fn new(config: Config) -> Self {
        let task_status: Arc<RwLock<TaskStatus>> = Arc::default();
        let executor = Executor::new(config, task_status.clone());
        Self {
            start_page: Default::default(),
            end_page: Default::default(),
            message: Default::default(),
            tab_type: TabType::DownloadPosts,
            with_pic: true,
            task_ongoing: false,
            period: 50,
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
                    ui.selectable_value(
                        &mut self.tab_type,
                        TabType::DownloadPosts,
                        "   从网络下载   ",
                    );
                    ui.selectable_value(
                        &mut self.tab_type,
                        TabType::ExportFromLocal,
                        "   从本地导出   ",
                    );
                    ui.selectable_value(&mut self.tab_type, TabType::About, "        关于        ");
                });

                if self.tab_type == TabType::About {
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
                } else {
                    if self.tab_type == TabType::DownloadPosts {
                        ui.checkbox(&mut self.with_pic, "同时下载图片");
                    } else {
                    }
                    ui.collapsing("高级设置", |ui| {
                        ui.horizontal(|ui| {
                            let hint = if self.tab_type == TabType::DownloadPosts {
                                ui.label("下载范围：");
                                "范围的单位为页，微博以页为单位返回\n每页大概15-20条博文"
                            } else {
                                ui.label("导出范围：");
                                "导出单位为条，按时间顺序排序，可选正序或逆序"
                            };

                            ui.add(
                                egui::TextEdit::singleline(&mut self.start_page)
                                    .desired_width(50.0),
                            )
                            .on_hover_text(hint);
                            ui.label("-");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.end_page).desired_width(50.0),
                            )
                            .on_hover_text(hint);
                        });
                        if self.tab_type == TabType::ExportFromLocal {
                            ui.add(egui::Slider::new(&mut self.period, 10..=200).text("每页"))
                                .on_hover_text("导出时默认50条博文分割为一个html文件");
                        }
                    });
                }
            });
            let start = self.start_page.parse::<u32>().unwrap_or(1);
            let end = self.end_page.parse::<u32>().unwrap_or(u32::MAX);
            ui.vertical_centered(|ui| {
                ui.set_enabled(!self.task_ongoing);
                if ui.button("开始").clicked() {
                    self.task_ongoing = true;
                    match self.tab_type {
                        TabType::DownloadPosts => {
                            self.executor.download_posts(start..=end, self.with_pic);
                        }
                        TabType::ExportFromLocal => {
                            self.executor.export_from_local(start..=end, false);
                        }
                        _ => {}
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
