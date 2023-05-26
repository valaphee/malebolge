use std::cell::RefCell;

use eframe::egui::{
    Align, Button, CentralPanel, Context, Frame, Grid, Id, Key, KeyboardShortcut, Layout,
    Modifiers, Ui, Vec2, WidgetText, Window,
};
use egui_dock::{DockArea, Node, Tree};

use crate::{
    gui::{assembly::AssemblyView, label::LabelView, raw::RawView},
    project::{DataViewType, Project},
};

mod assembly;
mod label;
mod raw;

#[derive(Default)]
pub struct App {
    project: Option<Project>,

    views: Tree<Box<dyn AppView>>,
    go_to_address_window: Option<GoToAddressWindow>,
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
        self.open_view(Box::new(LabelView));
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if self.project.is_some() {
            let open_view = CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    let mut context = AppContext {
                        project: self.project.as_mut().unwrap(),
                        open_view: RefCell::new(Default::default()),
                    };
                    DockArea::new(&mut self.views).show_inside(ui, &mut context);

                    // go to address window
                    if let Some(go_to_address_window) = &mut self.go_to_address_window {
                        if let Some(address) = go_to_address_window.ui(ui) {
                            context.open_address_view(address)
                        }
                        if !go_to_address_window.open() {
                            self.go_to_address_window = None;
                        }
                    }

                    // Ctrl+G: go to address
                    if ui.input_mut(|input| {
                        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::G))
                    }) && self.go_to_address_window.is_none()
                    {
                        self.go_to_address_window = Some(Default::default())
                    }

                    context.open_view.into_inner()
                })
                .inner;

            for view in open_view {
                self.open_view(view);
            }
            if self.views.is_empty() {
                self.project = None;
            }
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui
                        .add(Button::new("Open").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.project = Some(Project::new(path).unwrap());
                            self.open_label_view();
                        };
                    }
                    if ui
                        .add(Button::new("Exit").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        frame.close()
                    }
                });
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

pub struct AppContext<'a> {
    pub project: &'a mut Project,

    open_view: RefCell<Vec<Box<dyn AppView>>>,
}

impl AppContext<'_> {
    pub fn open_view(&self, view: Box<dyn AppView>) {
        self.open_view.borrow_mut().push(view);
    }

    pub fn open_address_view(&self, rva: u64) {
        if let Some(data_view) = self.project.data_view(rva) {
            match data_view.type_ {
                DataViewType::Raw => {
                    self.open_view(Box::new(RawView::new(rva, data_view)));
                }
                DataViewType::Assembly => {
                    self.open_view(Box::new(AssemblyView::new(rva, data_view)))
                }
            }
        }
    }
}

impl egui_dock::TabViewer for AppContext<'_> {
    type Tab = Box<dyn AppView>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(self, ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(std::ptr::addr_of!(tab))
    }
}

pub trait AppView {
    fn title(&self) -> String;

    fn ui(&mut self, context: &mut AppContext, ui: &mut Ui);
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
