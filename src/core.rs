use std::sync::{Arc, RwLock};

use anyhow;
use eframe::{
    egui::{self, vec2},
    NativeOptions,
};
use log::info;

use crate::executor::Executor;
use crate::login::{get_login_info, LoginState, Loginator};
use crate::message::TaskStatus;

pub enum MainState {
    Unlogined,
    Logining,
    Logged,
}

impl Default for MainState {
    fn default() -> Self {
        Self::Unlogined
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TabType {
    DownloadPosts,
    ExportFromLocal,
    About,
}

impl Default for TabType {
    fn default() -> Self {
        Self::DownloadPosts
    }
}

pub struct Core {
    state: MainState,
    task_status: Option<Arc<RwLock<TaskStatus>>>,
    executor: Option<Executor>,
    task_ongoing: bool,
    login_checked: bool,
    web_total: u32,
    db_total: u32,
    // variables associated with GUI
    web_start: u32,
    web_end: u32,
    db_start: u32,
    db_end: u32,
    message: String,
    tab_type: TabType,
    with_pic: bool,
    reverse: bool,
    image_definition: u8,
    period: u32,
    ratio: f32,
    // variable associated with logining GUI
    login_state: Option<Arc<RwLock<LoginState>>>,
    qrcode_img: Option<egui::TextureHandle>,
}

impl Default for Core {
    fn default() -> Self {
        Self {
            state: Default::default(),
            task_status: Default::default(),
            executor: Default::default(),
            task_ongoing: Default::default(),
            login_checked: Default::default(),
            web_total: Default::default(),
            db_total: Default::default(),
            web_start: 1,
            web_end: Default::default(),
            db_start: 1,
            db_end: Default::default(),
            message: Default::default(),
            tab_type: Default::default(),
            with_pic: true,
            reverse: true,
            image_definition: 2,
            period: 50,
            ratio: Default::default(),
            login_state: Default::default(),
            qrcode_img: Default::default(),
        }
    }
}

impl Core {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(self) -> anyhow::Result<()> {
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
        .unwrap();
        Ok(())
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
            MainState::Logged => self.when_logined(ctx, _frame),
        }
    }
}

impl Core {
    fn when_logined(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let task_status: Option<TaskStatus> = self
            .task_status
            .as_ref()
            .expect("core.status must be Some(_), bugs in there")
            .try_read()
            .ok()
            .map(|task_status| task_status.clone());
        if let Some(task_status) = task_status {
            match &task_status {
                TaskStatus::Init(web_total, db_total) => {
                    self.web_total = *web_total;
                    self.db_total = *db_total;
                    if self.db_end == 0 {
                        self.db_end = self.db_total;
                    }
                    if self.web_end == 0 {
                        self.web_end = self.web_total;
                    }
                    self.message = format!(
                        "账号共 {} 条收藏\n本地保存有 {} 条收藏",
                        self.web_total, self.db_total
                    );
                }
                TaskStatus::InProgress(ratio, msg) => {
                    self.ratio = *ratio;
                    self.message = msg.to_owned()
                }
                TaskStatus::Finished(web_total, db_total) => {
                    self.ratio = 1.;
                    self.task_ongoing = false;
                    self.web_total = *web_total;
                    self.db_total = *db_total;
                    self.message = format!(
                        "任务完成!\n账号剩 {} 条收藏\n本地保存有 {} 条收藏",
                        self.web_total, self.db_total
                    );
                }
                TaskStatus::Error(msg) => {
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
                        "    微博下载    ",
                    );
                    ui.selectable_value(
                        &mut self.tab_type,
                        TabType::ExportFromLocal,
                        "    本地微博    ",
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
                        if ui.button("对本地微博取消收藏").clicked() {
                            self.task_ongoing = true;
                            self.executor
                                .as_ref()
                                .expect("core.executor must be unwrapable, bugs in there")
                                .unfavorite_posts(1..=u32::MAX);
                        }
                    }
                    ui.collapsing("高级设置", |ui| {
                        ui.horizontal(|ui| {
                            if self.tab_type == TabType::DownloadPosts {
                                ui.label("下载范围：");
                            } else {
                                ui.label("导出范围：");
                            }

                            let (start, end, total, speed) =
                                if self.tab_type == TabType::DownloadPosts {
                                    (
                                        &mut self.web_start,
                                        &mut self.web_end,
                                        (self.web_total + 19) / 20 * 20,
                                        20,
                                    )
                                } else {
                                    (
                                        &mut self.db_start,
                                        &mut self.db_end,
                                        self.db_total,
                                        self.period,
                                    )
                                };

                            ui.add(egui::DragValue::new(start).clamp_range(1..=*end).speed(20));
                            ui.label("-");
                            ui.add(
                                egui::DragValue::new(end)
                                    .clamp_range(speed..=total)
                                    .speed(speed),
                            )
                        });
                        if self.tab_type == TabType::ExportFromLocal {
                            ui.add(egui::Slider::new(&mut self.period, 10..=200).text("每页"))
                                .on_hover_text("导出时默认50条博文分割为一个html文件");
                        }
                    });
                }
            });
            ui.vertical_centered(|ui| {
                ui.set_enabled(!self.task_ongoing);
                if ui.button("开始").clicked() {
                    self.task_ongoing = true;
                    match self.tab_type {
                        TabType::DownloadPosts => {
                            self.executor
                                .as_ref()
                                .expect("core.executor must be unwrapable, bugs in there")
                                .download_posts(
                                    self.web_start..=self.web_end,
                                    self.with_pic,
                                    self.image_definition,
                                );
                        }
                        TabType::ExportFromLocal => {
                            self.executor
                                .as_ref()
                                .expect("core.executor must be unwrapable, bugs in there")
                                .export_from_local(
                                    self.db_start..=self.db_start,
                                    self.reverse,
                                    self.image_definition,
                                );
                        }
                        _ => {}
                    }
                }
            });
            ui.add(egui::ProgressBar::new(self.ratio).show_percentage());
            ui.vertical_centered(|ui| {
                ui.label(&self.message);
            });
        });
    }

    fn when_unlogged(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let check_res = if !self.login_checked {
            self.login_checked = true;
            get_login_info().unwrap()
        } else {
            None
        };
        if let Some(login_info) = check_res {
            let task_status: Arc<RwLock<TaskStatus>> = Arc::default();
            let executor = Executor::new(login_info, task_status.clone());
            self.task_status = Some(task_status);
            self.executor = Some(executor);
            self.state = MainState::Logged;
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.vertical_centered(|ui| ui.heading("WeiBack"));
                    ui.label("还未登录，请重新登录！\n\n\n\n\n\n");
                    if ui.button("登录").clicked() {
                        self.state = MainState::Logining;
                    }
                });
            });
        }
    }

    fn when_logging(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let login_state = self
            .login_state
            .get_or_insert_with(|| {
                let login_state: Arc<RwLock<LoginState>> = Default::default();
                let res = login_state.clone();
                std::thread::spawn(move || {
                    let mut loginator = Loginator::new();
                    let qrcode = loginator.get_login_qrcode().unwrap();
                    *login_state.write().unwrap() = LoginState::QRCodeGotten(qrcode);
                    loginator.wait_confirm().unwrap();
                    *login_state.write().unwrap() = LoginState::Confirmed;
                    let login_info = loginator.wait_login().unwrap();
                    *login_state.write().unwrap() = LoginState::Logged(login_info);
                });
                res
            })
            .try_read()
            .ok()
            .map(|a| a.clone());
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(login_state) = login_state {
                ui.vertical_centered(|ui| match login_state {
                    LoginState::GettingQRCode => {
                        ui.label("正在获取二维码，稍等...");
                    }
                    LoginState::QRCodeGotten(image_data) => {
                        let qrcode: &_ = self.qrcode_img.get_or_insert_with(|| {
                            ui.ctx()
                                .load_texture("login_qrcode", image_data, Default::default())
                        });
                        ui.image(qrcode, qrcode.size_vec2());
                        ui.label("请尽快用手机扫描二维码并确认");
                    }
                    LoginState::Confirmed => {
                        ui.label("扫码成功，登录中...");
                    }
                    LoginState::Logged(login_info) => {
                        let task_status: Arc<RwLock<TaskStatus>> = Arc::default();
                        let executor = Executor::new(login_info, task_status.clone());
                        self.task_status = Some(task_status);
                        self.executor = Some(executor);
                        self.state = MainState::Logged;
                        self.qrcode_img = None;
                        self.login_state = None;
                    }
                });
            }
        });
    }
}
