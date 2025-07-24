pub mod options;
pub mod task_handler;
pub mod task_proxy;

use log::info;
use tokio::sync::mpsc::{Receiver, channel, error::TryRecvError};

use crate::error::Result;
use crate::message::Message;
use crate::ui::UI;

pub use options::{TaskOptions, UserPostFilter};
pub use task_handler::{TaskHandler, TaskRequest};
pub use task_proxy::TaskProxy;

pub struct Core {
    task_executor: TaskProxy,
    msg_receiver: Receiver<Message>,
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
        let task_status: Option<Message> = match self.msg_receiver.try_recv() {
            Ok(status) => Some(status),
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!("{}", e),
        };

        if let Some(task_status) = task_status {
            match task_status {
                Message::TaskProgress(tp) => {}
                Message::UserMeta(um) => {}
                Message::Err(msg) => {
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
