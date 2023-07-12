use std::sync::{Arc, RwLock};

use eframe::{
    egui::{self, vec2},
    NativeOptions,
};
use log::info;

use crate::config::get_config;
use crate::executor::Executor;
use crate::message::TaskStatus;

pub enum MainState {
    Unlogined,
    Logining,
    Logined,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TabType {
    DownloadPosts,
    ExportFromLocal,
    About,
}

pub struct Core {
    state: MainState,
    task_status: Arc<RwLock<TaskStatus>>,
    executor: Executor,
    task_ongoing: bool,
    // variables associated with GUI
    start_page: String,
    end_page: String,
    message: String,
    tab_type: TabType,
    with_pic: bool,
    reverse: bool,
    image_definition: u8,
    period: u32,
    ratio: f32,
}

impl Core {
    pub fn new() -> Self {
        let config = get_config().unwrap();
        let task_status: Arc<RwLock<TaskStatus>> = Arc::default();
        let executor = Executor::new(config, task_status.clone());
        Self {
            state: MainState::Unlogined,
            task_status,
            executor,
            task_ongoing: false,
            // variables associated with GUI
            start_page: "1".into(),
            end_page: u32::MAX.to_string(),
            message: "Hello!".into(),
            tab_type: TabType::DownloadPosts,
            with_pic: true,
            reverse: true,
            image_definition: 2,
            period: 50,
            ratio: 0.0,
        }
    }

    pub fn run(self) {
        info!("starting gui...");
        eframe::run_native(
            "weiback",
            NativeOptions {
                initial_window_size: Some(vec2(300.0, 300.0)),
                ..Default::default()
            },
            Box::new(|cc| {
                set_font(cc);
                Box::new(self)
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

impl eframe::App for Core {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.state {
            MainState::Unlogined => self.when_unlogged(ctx, _frame),
            MainState::Logining => self.when_logging(ctx, _frame),
            MainState::Logined => self.when_logined(ctx, _frame),
        }
    }
}

impl Core {
    fn when_logined(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    self.ratio = 1.;
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
                    ui.horizontal(|ui| {
                        ui.label("图片清晰度：");
                        ui.selectable_value(&mut self.image_definition, 0, "最低");
                        ui.selectable_value(&mut self.image_definition, 1, "中等");
                        ui.selectable_value(&mut self.image_definition, 2, "最高");
                    });
                    if self.tab_type == TabType::DownloadPosts {
                        ui.checkbox(&mut self.with_pic, "同时下载图片");
                    } else {
                        ui.checkbox(&mut self.reverse, "按时间逆序")
                            .on_hover_text("时间逆序即最上方的微博为最新的微博");
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
                            self.executor.download_posts(
                                start..=end,
                                self.with_pic,
                                self.image_definition,
                            );
                        }
                        TabType::ExportFromLocal => {
                            self.executor.export_from_local(
                                start..=end,
                                self.reverse,
                                self.image_definition,
                            );
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

    fn when_unlogged(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {}

    fn when_logging(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {}
}
