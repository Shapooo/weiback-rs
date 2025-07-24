pub mod tabs;

use std::any::Any;
use std::time::Duration;

use eframe::{
    NativeOptions,
    egui::{vec2, viewport::ViewportBuilder},
};

use crate::core::TaskRequest;
use tabs::{
    Tab, about_tab::AboutTab, backup_fav_tab::BackupFavTab, backup_user_tab::BackupUserTab,
    export_from_local_tab::ExportFromLocalTab,
};

pub struct UI {
    pub tabs: Vec<Box<dyn Tab>>,
    pub current_tab_idx: usize,
    pub task_ongoing: bool,
    pub message: String,
    pub ratio: f32,
    pub web_total: u32,
    pub db_total: u32,
}

impl Default for UI {
    fn default() -> Self {
        let tabs: Vec<Box<dyn Tab>> = vec![
            Box::<BackupFavTab>::default(),
            Box::<BackupUserTab>::default(),
            Box::<ExportFromLocalTab>::default(),
            Box::<AboutTab>::default(),
        ];
        Self {
            tabs,
            current_tab_idx: 0,
            task_ongoing: false,
            message: "请开始任务".into(),
            ratio: 0.0,
            web_total: 0,
            db_total: 0,
        }
    }
}

impl UI {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_options() -> NativeOptions {
        NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size(vec2(330., 300.)),
            ..Default::default()
        }
    }

    pub fn set_context(&self, cc: &eframe::CreationContext) {
        set_font(cc);
    }

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) -> Option<TaskRequest> {
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
                    }
                });
            });
            ui.add(egui::ProgressBar::new(self.ratio).show_percentage());
            ui.vertical_centered(|ui| {
                ui.label(&self.message);
            });
        });
        ctx.request_repaint_after(Duration::from_millis(200));
        Some(TaskRequest::UnfavoritePosts)
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

// Trait object `dyn Tab` doesn't have a constant size known at compile-time,
// so we need to implement `as_any_mut` to downcast it.
pub trait AsAny: 'static {
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

    fn ui(&mut self, ui: &mut egui::Ui) -> Option<TaskRequest> {
        self.as_mut().ui(ui)
    }
}
