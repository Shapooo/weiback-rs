mod tabs;
mod task_proxy;

use std::any::Any;
use std::time::Duration;

use anyhow;
use eframe::{
    NativeOptions,
    egui::{vec2, viewport::ViewportBuilder},
};
use log::info;
use tokio::sync::mpsc::{Receiver, channel, error::TryRecvError};

use crate::error::Result;
use crate::message::Message;
use crate::task_handler::Task;
use tabs::{
    Tab, about_tab::AboutTab, backup_fav_tab::BackupFavTab, backup_user_tab::BackupUserTab,
    export_from_local_tab::ExportFromLocalTab,
};
use task_proxy::TaskProxy;

pub struct Core {
    tabs: Vec<Box<dyn Tab>>,
    current_tab_idx: usize,

    task_status_receiver: Receiver<Result<Message>>,
    executor: TaskProxy,
    task_ongoing: bool,

    // View data
    message: String,
    ratio: f32,
    web_total: u32,
    db_total: u32,
}

impl Default for Core {
    fn default() -> Self {
        let tabs: Vec<Box<dyn Tab>> = vec![
            Box::<BackupFavTab>::default(),
            Box::<BackupUserTab>::default(),
            Box::<ExportFromLocalTab>::default(),
            Box::<AboutTab>::default(),
        ];
        let (task_status_sender, task_status_receiver) = channel(100);
        Self {
            tabs,
            current_tab_idx: 0,
            task_status_receiver,
            executor: TaskProxy::new(task_status_sender),
            task_ongoing: false,
            message: "请开始任务".into(),
            ratio: 0.0,
            web_total: 0,
            db_total: 0,
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
                viewport: ViewportBuilder::default().with_inner_size(vec2(330., 300.)),
                ..Default::default()
            },
            Box::new(|cc| {
                set_font(cc);
                Ok(Box::new(self))
            }),
        )
        .unwrap();
        Ok(())
    }

    fn handle_task_responses(&mut self) {
        let task_status: Option<Result<Message>> = match self.task_status_receiver.try_recv() {
            Ok(status) => Some(status),
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!("{}", e),
        };

        if let Some(task_status) = task_status {
            match task_status {
                Ok(Message::SumOfFavDB(web_total, db_total)) => {
                    self.web_total = web_total;
                    self.db_total = db_total;
                    if let Some(tab) = self.tabs[0].as_any_mut().downcast_mut::<BackupFavTab>() {
                        tab.set_web_total(web_total);
                    }
                    if let Some(tab) = self.tabs[2]
                        .as_any_mut()
                        .downcast_mut::<ExportFromLocalTab>()
                    {
                        tab.set_db_total(db_total);
                    }
                    self.message = format!(
                        "账号共 {} 条收藏\n本地保存有 {} 条收藏",
                        self.web_total, self.db_total
                    );
                }
                Ok(Message::InProgress(ratio, msg)) => {
                    self.ratio = ratio;
                    self.message = msg;
                }
                Ok(Message::Finished(web_total, db_total)) => {
                    self.ratio = 1.;
                    self.task_ongoing = false;
                    self.web_total = web_total;
                    self.db_total = db_total;
                    self.message = format!(
                        "任务完成!\n账号剩 {} 条收藏\n本地保存有 {} 条收藏",
                        self.web_total, self.db_total
                    );
                }
                Ok(Message::UserMeta(_id, _screen_name, _avatar)) => {
                    // TODO: how to show user meta?
                }
                Err(msg) => {
                    self.task_ongoing = false;
                    self.message = msg.to_string();
                }
            }
        }
    }
}

fn set_font(cc: &eframe::CreationContext) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "source".into(),
        egui::FontData::from_static(include_bytes!("../../res/fonts/SourceHanSansCN-Medium.otf"))
            .into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("source".into());
    cc.egui_ctx.set_fonts(fonts);
}

impl eframe::App for Core {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_task_responses();
        self.when_logined(ctx, frame);
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

impl Core {
    fn when_logined(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| ui.heading("WeiBack"));
            ui.add_enabled_ui(!self.task_ongoing, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        for (i, tab) in self.tabs.iter().enumerate() {
                            ui.selectable_value(&mut self.current_tab_idx, i, tab.name());
                        }
                    });

                    let tab = &mut self.tabs[self.current_tab_idx];
                    if let Some(task) = tab.ui(ui) {
                        self.task_ongoing = true;
                        self.ratio = 0.0;
                        self.message = "任务开始...".into();
                        self.executor.send_task(task);
                    }
                });
            });
            ui.add(egui::ProgressBar::new(self.ratio).show_percentage());
            ui.vertical_centered(|ui| {
                ui.label(&self.message);
            });
        });
    }
}

// Trait object `dyn Tab` doesn't have a constant size known at compile-time,
// so we need to implement `as_any_mut` to downcast it.
trait AsAny: 'static {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> AsAny for T {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Tab for Box<dyn Tab> {
    fn name(&self) -> &str {
        self.as_ref().name()
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Option<Task> {
        self.as_mut().ui(ui)
    }
}
