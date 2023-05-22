use eframe::egui::{
    Align, Button, CentralPanel, Context, Frame, Grid, Key, KeyboardShortcut, Layout, Modifiers,
    Ui, Vec2, WidgetText, Window,
};
use egui_dock::{DockArea, Node, Tree};

use crate::{
    client::{
        assembly::AssemblyView,
        label::{LabelView, LabelWindow},
        process::AttachProcessWindow,
        raw::RawView,
    },
    project::{Project, SectionType},
};

mod assembly;
mod label;
mod process;
mod raw;

#[derive(Default)]
pub struct App {
    attach_process_window: Option<AttachProcessWindow>,

    context: Option<AppContext>,
    views: Tree<Box<dyn AppView>>,
}

impl App {
    fn open_view(&mut self, view: Box<dyn AppView>) {
        let title = view.title();
        if let Some((node_index, tab_index)) =
            self.views
                .iter()
                .enumerate()
                .find_map(|(node_index, node)| match node {
                    Node::Leaf { tabs, .. } => {
                        if let Some(tab_index) = tabs.iter().position(|tab| tab.title() == title) {
                            Some((node_index, tab_index))
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
        {
            self.views.set_focused_node(node_index.into());
            self.views
                .set_active_tab(node_index.into(), tab_index.into());
        } else {
            self.views.push_to_first_leaf(view)
        }
    }

    fn open_label_view(&mut self) {
        self.open_view(Box::new(LabelView::default()));
    }

    fn open_address_view(&mut self, address: u64) {
        let Some(section) = self.context.as_ref().unwrap().project.section(address) else {
            return;
        };
        match section.type_ {
            SectionType::Raw => {
                self.open_view(Box::new(RawView::new(address)));
            }
            SectionType::Assembly => {
                self.open_view(Box::new(AssemblyView::new(address)));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if let Some(context) = &mut self.context {
            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    DockArea::new(&mut self.views).show_inside(ui, context);

                    // check go to address window
                    if let Some(go_to_address_window) = &mut context.go_to_address_window {
                        if let Some(address) = go_to_address_window.ui(ui) {
                            context.go_to_address = Some(address);
                        }
                        if !go_to_address_window.open() {
                            context.go_to_address_window = None;
                        }
                    }

                    // check label window
                    if let Some(label_window) = &mut context.label_window {
                        if let Some(label) = label_window.ui(ui) {
                            context.project.labels.insert(label.0, label.1);
                        }
                        if !label_window.open() {
                            context.label_window = None;
                        }
                    }

                    // Ctrl+G: go to address
                    if ui.input_mut(|input| {
                        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::G))
                    }) && context.go_to_address_window.is_none()
                    {
                        context.go_to_address_window = Some(Default::default())
                    }
                });

            if let Some(address) = context.go_to_address {
                context.go_to_address = None;
                self.open_address_view(address);
            }

            if self.views.is_empty() {
                self.context = None;
            }
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui
                        .add(Button::new("Open File").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        let Some(path) = rfd::FileDialog::new().pick_file() else {
                            todo!()
                        };
                        self.context = Some(AppContext {
                            project: Project::create_from_file(path).unwrap(),
                            go_to_address: None,
                            go_to_address_window: None,
                            label_window: None,
                        });
                        self.open_label_view();
                    }
                    if ui
                        .add(Button::new("Attach Process").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        self.attach_process_window = Some(AttachProcessWindow::new());
                    }
                    if ui
                        .add(Button::new("Exit").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        frame.close()
                    }
                });

                // check attach process window
                if let Some(attach_process_window) = &mut self.attach_process_window {
                    if let Some(pid) = attach_process_window.ui(ui) {
                        self.context = Some(AppContext {
                            project: Project::create_from_process(pid).unwrap(),
                            go_to_address: None,
                            go_to_address_window: None,
                            label_window: None,
                        });
                        self.open_label_view();
                    } else if !attach_process_window.open {
                        self.attach_process_window = None;
                    }
                }
            });
        }

        // F11: toggle fullscreen
        if ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F11))
        }) {
            frame.set_fullscreen(!frame.info().window_info.fullscreen)
        }
    }
}

struct AppContext {
    project: Project,

    go_to_address: Option<u64>,
    go_to_address_window: Option<GoToAddressWindow>,
    label_window: Option<LabelWindow>,
}

impl egui_dock::TabViewer for AppContext {
    type Tab = Box<dyn AppView>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(self, ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }
}

trait AppView {
    fn title(&self) -> String;

    fn ui(&mut self, viewer: &mut AppContext, ui: &mut Ui);
}

struct GoToAddressWindow {
    open: bool,
    address: String,
}

impl GoToAddressWindow {
    fn ui(&mut self, ui: &mut Ui) -> Option<u64> {
        let mut address = None;
        let mut close = false;
        Window::new("Go To Address")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                Grid::new("").num_columns(2).show(ui, |ui| {
                    ui.label("Address");
                    ui.text_edit_singleline(&mut self.address);
                    ui.end_row();
                });
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if ui.button("Go").clicked() {
                        if let Ok(address_) = u64::from_str_radix(&self.address, 16) {
                            address = Some(address_);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                })
            });
        if address.is_some() || close {
            self.open = false;
        }
        address
    }

    pub fn open(&self) -> bool {
        self.open
    }
}

impl Default for GoToAddressWindow {
    fn default() -> Self {
        Self {
            open: true,
            address: Default::default(),
        }
    }
}
