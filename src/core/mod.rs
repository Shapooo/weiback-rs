pub mod options;
pub mod task_handler;
pub mod task_proxy;

use log::info;
use tokio::sync::mpsc::{Receiver, channel, error::TryRecvError};

use crate::error::Result;
use crate::message::Message;
use crate::ui::{
    AsAny, UI,
    tabs::{backup_fav_tab::BackupFavTab, export_from_local_tab::ExportFromLocalTab},
};

pub use options::{TaskOptions, UserPostFilter};
pub use task_handler::{Task, TaskHandler};
pub use task_proxy::TaskProxy;

pub struct Core {
    task_executor: TaskProxy,
    msg_receiver: Receiver<Result<Message>>,
    ui: UI,
}

impl Core {
    pub fn new() -> Self {
        let (msg_sender, msg_receiver) = channel(100);
        Self {
            task_executor: TaskProxy::new(msg_sender),
            msg_receiver,
            ui: UI::new(),
        }
    }

    pub fn run(self) -> Result<()> {
        info!("starting gui...");
        eframe::run_native(
            "weiback",
            UI::default_options(),
            Box::new(|cc| {
                self.ui.set_context(cc);
                Ok(Box::new(self))
            }),
        )
        .unwrap();
        Ok(())
    }

    fn handle_task_responses(&mut self) {
        let task_status: Option<Result<Message>> = match self.msg_receiver.try_recv() {
            Ok(status) => Some(status),
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!("{}", e),
        };

        if let Some(task_status) = task_status {
            match task_status {
                Ok(Message::SumOfFavDB(web_total, db_total)) => {
                    self.ui.web_total = web_total;
                    self.ui.db_total = db_total;
                    if let Some(tab) = self.ui.tabs[0].as_any_mut().downcast_mut::<BackupFavTab>() {
                        tab.set_web_total(web_total);
                    }
                    if let Some(tab) = self.ui.tabs[2]
                        .as_any_mut()
                        .downcast_mut::<ExportFromLocalTab>()
                    {
                        tab.set_db_total(db_total);
                    }
                    self.ui.message = format!(
                        "账号共 {} 条收藏\n本地保存有 {} 条收藏",
                        self.ui.web_total, self.ui.db_total
                    );
                }
                Ok(Message::InProgress(ratio, msg)) => {
                    self.ui.ratio = ratio;
                    self.ui.message = msg;
                }
                Ok(Message::Finished(web_total, db_total)) => {
                    self.ui.ratio = 1.;
                    self.ui.task_ongoing = false;
                    self.ui.web_total = web_total;
                    self.ui.db_total = db_total;
                    self.ui.message = format!(
                        "任务完成!\n账号剩 {} 条收藏\n本地保存有 {} 条收藏",
                        self.ui.web_total, self.ui.db_total
                    );
                }
                Ok(Message::UserMeta(_id, _screen_name, _avatar)) => {
                    // TODO: how to show user meta?
                }
                Err(msg) => {
                    self.ui.task_ongoing = false;
                    self.ui.message = msg.to_string();
                }
            }
        }
    }
}

impl eframe::App for Core {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_task_responses();
        if let Some(task) = self.ui.update(ctx, frame) {
            self.task_executor.send_task(task);
        }
    }
}
