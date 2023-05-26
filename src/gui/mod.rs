use eframe::egui::{
    Align, Button, CentralPanel, Context, Frame, Grid, Key, KeyboardShortcut, Layout, Modifiers,
    Ui, Vec2, WidgetText, Window,
};
use egui_dock::{DockArea, Node, Tree};

use crate::{gui::label::LabelView, project::Project};

mod assembly;
mod label;
mod raw;

#[derive(Default)]
pub struct App {
    context: Option<AppContext>,

    views: Tree<Box<dyn AppView>>,
    go_to_address_window: Option<GoToAddressWindow>,
}

impl App {
    fn open_view(views: &mut Tree<Box<dyn AppView>>, view: Box<dyn AppView>) {
        let title = view.title();
        if let Some((node_index, tab_index)) =
            views
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
            views.set_focused_node(node_index.into());
            views.set_active_tab(node_index.into(), tab_index.into());
        } else {
            views.push_to_first_leaf(view)
        }
    }

    fn open_label_view(views: &mut Tree<Box<dyn AppView>>) {
        Self::open_view(views, Box::new(LabelView::default()));
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if let Some(context) = &mut self.context {
            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    DockArea::new(&mut self.views).show_inside(ui, context);

                    // go to address window
                    if let Some(go_to_address_window) = &mut self.go_to_address_window {
                        if let Some(_address) = go_to_address_window.ui(ui) {
                            // TODO context.open_view.push(Box::new());
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
                });

            while let Some(view) = context.open_view.pop() {
                App::open_view(&mut self.views, view);
            }
            if self.views.is_empty() {
                self.context = None;
            }
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui
                        .add(Button::new("Open").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.context = Some(AppContext::new(Project::new(path).unwrap()));
                            Self::open_label_view(&mut self.views);
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

struct AppContext {
    project: Project,

    open_view: Vec<Box<dyn AppView>>,
}

impl AppContext {
    fn new(project: Project) -> Self {
        Self {
            project,
            open_view: Default::default(),
        }
    }
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
