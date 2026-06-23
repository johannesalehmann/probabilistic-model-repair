use iced::futures::FutureExt;
use iced::widget::button::Status;
use iced::widget::container::{Style, bordered_box};
use iced::widget::pane_grid::{Axis, Direction, Pane};
use iced::widget::tooltip::Position;
use iced::widget::{
    Container, PaneGrid, Row, Space, button, column, container, pane_grid, row, stack, text,
};
use iced::widget::{image, tooltip};
use iced::{Element, Task};
use iced_core::border::Radius;
use iced_core::image::Handle;
use iced_core::text::Wrapping;
use iced_core::widget::Id;
use iced_core::{Background, Border, Color, Length, Padding, Point, Rectangle};
use std::collections::HashMap;
use std::time::Duration;

const BACKGROUND_COLOR: Color = Color {
    r: 255.0 / 256.0,
    g: 185.0 / 256.0,
    b: 99.0 / 256.0,
    a: 1.0,
};
const PANE_SPACING: f32 = 5.0;
const PANE_RADIUS: f32 = 5.0;

pub struct TabbedWorkspace<W: Window> {
    selected_window: pane_grid::Pane,
    pane_grid_state: pane_grid::State<TabView<W>>,
    id_to_pane: HashMap<Id, Pane>,
    hover_preview_pane: Option<Pane>,
}

impl<W: Window> TabbedWorkspace<W> {
    pub fn new() -> Self {
        let (default_window, default_id) = TabView::new();
        let (pane_grid_state, selected_window) = pane_grid::State::new(default_window);
        let mut id_to_pane = HashMap::new();
        id_to_pane.insert(default_id, selected_window);
        Self {
            pane_grid_state,
            selected_window,
            id_to_pane,
            hover_preview_pane: None,
        }
    }

    fn clear_preview(&mut self) {
        if let Some(old_preview) = self.hover_preview_pane {
            self.pane_grid_state
                .get_mut(old_preview)
                .unwrap()
                .hover_preview = None;
            self.hover_preview_pane = None;
        }
    }

    fn get_hover_location(
        &self,
        cursor: Point,
        zones: &[(Id, Rectangle)],
    ) -> Option<(Pane, PaneDragKind)> {
        for (id, rect) in zones {
            if let Some(&pane) = self.id_to_pane.get(id) {
                let x_progress = (cursor.x - rect.x) / rect.width;
                let y_progress = (cursor.y - rect.y) / rect.height;

                let left_distance = x_progress;
                let right_distance = 1.0 - x_progress;
                let top_distance = y_progress;
                let bottom_distance = 1.0 - y_progress;

                let threshold = 0.15;

                let kind = if left_distance <= threshold
                    && left_distance <= top_distance
                    && left_distance <= bottom_distance
                {
                    SplitKind::Left
                } else if right_distance <= threshold
                    && right_distance <= top_distance
                    && right_distance <= bottom_distance
                {
                    SplitKind::Right
                } else if top_distance <= threshold
                    && top_distance <= left_distance
                    && top_distance <= right_distance
                {
                    SplitKind::Top
                } else if bottom_distance <= threshold
                    && bottom_distance <= left_distance
                    && bottom_distance <= right_distance
                {
                    SplitKind::Bottom
                } else {
                    SplitKind::Center
                };

                return Some((pane, PaneDragKind::Split { kind }));
            }
        }
        None
    }

    pub fn update(&mut self, message: Message<W::TabAction>) -> Task<Message<W::TabAction>> {
        match message {
            Message::PaneGridResized(event) => {
                self.pane_grid_state.resize(event.split, event.ratio);
            }
            Message::SelectTab { pane, tab } => {
                self.pane_grid_state.get_mut(pane).unwrap().selected = tab;
            }
            Message::CloseTab { pane: pane_id, tab } => {
                let pane = self.pane_grid_state.get_mut(pane_id).unwrap();
                pane.tabs.remove(tab);
                if pane.selected > 0 && pane.selected >= tab {
                    pane.selected -= 1;
                }

                if pane.tabs.is_empty() {
                    self.pane_grid_state.close(pane_id);
                    // TODO: Clean up IDs
                }
            }
            Message::DropTab {
                old_pane,
                tab_index,
                cursor,
                bounds,
            } => {
                return iced_drop::zones_on_point(
                    move |zones| Message::HandleDropZones {
                        old_pane,
                        tab_index,
                        zones,
                        cursor,
                    },
                    cursor,
                    None,
                    None,
                );
            }

            Message::Drag {
                old_pane,
                tab_index,
                cursor,
                bounds,
            } => {
                return iced_drop::zones_on_point(
                    move |zones| Message::HandleHoverZones {
                        old_pane,
                        tab_index,
                        zones,
                        cursor,
                    },
                    cursor,
                    None,
                    None,
                );
            }
            Message::HandleDropZones {
                old_pane: old_pane_index,
                tab_index,
                cursor,
                zones,
            } => {
                self.clear_preview();
                if let Some((pane, drop_kind)) = self.get_hover_location(cursor, &zones[..]) {
                    let old_pane = self.pane_grid_state.get_mut(old_pane_index).unwrap();
                    let tab = old_pane.tabs.remove(tab_index);
                    if old_pane.selected >= tab_index && tab_index > 0 {
                        old_pane.selected -= 1;
                    }
                    match drop_kind {
                        PaneDragKind::Tab { index } => {
                            let new_pane = self.pane_grid_state.get_mut(pane).unwrap();
                            new_pane.selected = index;
                            new_pane.tabs.insert(index, tab);
                        }
                        PaneDragKind::Split {
                            kind: SplitKind::Center,
                        } => {
                            let new_pane = self.pane_grid_state.get_mut(pane).unwrap();
                            new_pane.selected = new_pane.tabs.len();
                            new_pane.tabs.push(tab);
                        }
                        PaneDragKind::Split { kind } => {
                            let axis = match kind {
                                SplitKind::Top | SplitKind::Bottom => Axis::Horizontal,
                                SplitKind::Left | SplitKind::Right => Axis::Vertical,
                                SplitKind::Center => unreachable!(),
                            };
                            let (mut new_tab_view, id) = TabView::new();
                            new_tab_view.tabs.push(tab);
                            let (split_result, _) = self
                                .pane_grid_state
                                .split(axis, pane, new_tab_view)
                                .unwrap();
                            self.id_to_pane.insert(id, split_result);
                            if kind == SplitKind::Top || kind == SplitKind::Left {
                                self.pane_grid_state.swap(pane, split_result);
                            }
                        }
                    }
                    let old_pane = self.pane_grid_state.get_mut(old_pane_index).unwrap();
                    if old_pane.tabs.len() == 0 {
                        let result = self.pane_grid_state.close(old_pane_index);
                        if result.is_some() {
                            // TODO: Clean up ids.
                        }
                    }
                }
            }

            Message::HandleHoverZones {
                old_pane,
                tab_index,
                cursor,
                zones,
            } => {
                self.clear_preview();
                if let Some((pane, drop_kind)) = self.get_hover_location(cursor, &zones[..]) {
                    let new_pane = self.pane_grid_state.get_mut(pane).unwrap();
                    new_pane.hover_preview = Some(drop_kind);
                    self.hover_preview_pane = Some(pane);
                }
            }

            Message::TabAction {
                pane,
                tab_index,
                action,
            } => {
                let pane = self.pane_grid_state.get_mut(pane).unwrap();
                let tab = &mut pane.tabs[tab_index];
                tab.update(action);
            }
            Message::CancelDrag => {
                println!("Cancelled drag!");
                self.clear_preview()
            }
        }
        Task::none()
    }

    pub fn view<'a, Msg: 'a, F: 'a + Clone + Fn(Message<W::TabAction>) -> Msg>(
        &'a self,
        emit_message: F,
    ) -> Element<'a, Msg> {
        Element::map(
            container(
                PaneGrid::new(&self.pane_grid_state, |id, pane, maximised| {
                    let above = self.pane_grid_state.adjacent(id, Direction::Up).is_some();
                    let below = self.pane_grid_state.adjacent(id, Direction::Down).is_some();
                    let left = self.pane_grid_state.adjacent(id, Direction::Left).is_some();
                    let right = self
                        .pane_grid_state
                        .adjacent(id, Direction::Right)
                        .is_some();
                    pane_grid::Content::new(pane.view(id, above, below, left, right))
                })
                .on_resize(3, Message::PaneGridResized)
                .spacing(PANE_SPACING),
            )
            .style(|t| container::Style::default().background(BACKGROUND_COLOR))
            .into(),
            emit_message,
        )
    }

    pub fn open_window(&mut self, window: W) {
        self.pane_grid_state
            .get_mut(self.selected_window)
            .unwrap()
            .tabs
            .push(window)
    }
}

pub trait Window {
    type TabAction: Clone + Send + 'static;
    fn title(&self) -> String;
    fn icon(&self) -> Option<Handle>;
    fn update(&mut self, action: Self::TabAction);
    fn view<'a>(&'a self) -> Element<'a, Self::TabAction>;
}

#[derive(Clone)]
pub enum Message<TabAction> {
    PaneGridResized(pane_grid::ResizeEvent),
    SelectTab {
        pane: Pane,
        tab: usize,
    },
    CloseTab {
        pane: Pane,
        tab: usize,
    },

    DropTab {
        old_pane: Pane,
        tab_index: usize,
        cursor: iced::Point,
        bounds: iced::Rectangle,
    },
    HandleDropZones {
        old_pane: Pane,
        tab_index: usize,
        cursor: iced::Point,
        zones: Vec<(Id, iced::Rectangle)>,
    },
    Drag {
        old_pane: Pane,
        tab_index: usize,
        cursor: iced::Point,
        bounds: iced::Rectangle,
    },
    HandleHoverZones {
        old_pane: Pane,
        tab_index: usize,
        cursor: iced::Point,
        zones: Vec<(Id, iced::Rectangle)>,
    },
    CancelDrag,

    TabAction {
        pane: Pane,
        tab_index: usize,
        action: TabAction,
    },
}

enum PaneDragKind {
    Split { kind: SplitKind },
    Tab { index: usize },
}

#[derive(Eq, PartialEq)]
enum SplitKind {
    Center,
    Top,
    Bottom,
    Left,
    Right,
}

struct TabView<W: Window> {
    tabs: Vec<W>,
    selected: usize,
    id: Id,
    hover_preview: Option<PaneDragKind>,
}

impl<W: Window> TabView<W> {
    pub fn new() -> (Self, Id) {
        let id = Id::unique();
        (
            Self {
                tabs: Vec::new(),
                selected: 0,
                id: id.clone(),
                hover_preview: None,
            },
            id,
        )
    }

    pub fn view<'a>(
        &'a self,
        pane: Pane,
        above: bool,
        below: bool,
        left: bool,
        right: bool,
    ) -> Element<'a, Message<W::TabAction>> {
        let mut tab_bar = Row::new().spacing(5.0);
        if !above {
            tab_bar = tab_bar.padding(Padding::default().top(PANE_SPACING))
        }
        for (tab_index, tab) in self.tabs.iter().enumerate() {
            let image: Option<Element<_, _, _>> = tab.icon().map(|h| {
                container(image::Image::new(h).width(10).height(10).border_radius(4))
                    .center_y(Length::Fill)
                    .into()
            });
            let title = tab.title();
            let text = container(text!("{}", title).wrapping(Wrapping::None))
                .width(Length::Fill)
                .clip(true)
                .padding(Padding::new(0.0).top(-2.0));
            let tooltip_content =
                container(text!("{}", title))
                    .padding(6)
                    .style(|t| container::Style {
                        text_color: Some(Color::BLACK),
                        background: Some(Background::Color(Color::from_rgb(0.8, 0.8, 0.8))),
                        border: Border {
                            color: Default::default(),
                            width: 0.0,
                            radius: Radius::new(4),
                        },
                        shadow: Default::default(),
                        snap: false,
                    });
            let text = tooltip(text, tooltip_content, Position::Bottom)
                .delay(Duration::from_secs_f64(0.2));
            let x = button(text!("×").center())
                .style(|theme, status| {
                    let lightness = match status {
                        Status::Active | Status::Disabled => 1.0,
                        Status::Hovered => 0.8,
                        Status::Pressed => 0.9,
                    };
                    let color = Color::from_rgb(lightness, lightness, lightness);
                    button::Style {
                        background: Some(Background::Color(color)),
                        text_color: Color::BLACK,
                        border: Border {
                            color,
                            width: 0.0,
                            radius: Radius::new(7.5),
                        },
                        shadow: Default::default(),
                        snap: false,
                    }
                })
                .on_press(Message::CloseTab {
                    pane,
                    tab: tab_index,
                })
                .height(15)
                .width(15)
                .padding(0);
            let x = container(x).center_y(Length::Fill);
            let row = match image {
                Some(image) => row![image, text, x],
                None => row![image, text, x],
            }
            .padding(4)
            .spacing(4)
            .width(110)
            .height(26);
            let header = Container::new(row).style(|t| container::Style {
                text_color: Some(Color::BLACK),
                background: Some(Background::Color(Color::WHITE)),
                border: Border {
                    color: Default::default(),
                    width: 0.0,
                    radius: Radius {
                        top_left: PANE_RADIUS,
                        top_right: PANE_RADIUS,
                        bottom_right: 0.0,
                        bottom_left: 0.0,
                    },
                },
                shadow: Default::default(),
                snap: false,
            });
            let selected_color = if self.selected == tab_index {
                Color::WHITE
            } else {
                BACKGROUND_COLOR
            };
            let selected_bar = container(Space::new())
                .width(110)
                .height(3)
                .style(move |_| container::Style {
                    text_color: None,
                    background: Some(Background::Color(selected_color)),
                    border: Default::default(),
                    shadow: Default::default(),
                    snap: false,
                });

            let header = column![header, selected_bar];

            let droppable = iced_drop::droppable(header)
                .on_drop(move |location, bounds| Message::DropTab {
                    old_pane: pane,
                    tab_index,
                    cursor: location,
                    bounds,
                })
                .on_drag(move |location, bounds| Message::Drag {
                    old_pane: pane,
                    tab_index,
                    cursor: location,
                    bounds,
                })
                .on_cancel(Message::CancelDrag)
                .on_click(Message::SelectTab {
                    pane,
                    tab: tab_index,
                })
                .drag_hide(true);
            tab_bar = tab_bar.push(droppable);
        }

        let tab_bar = Container::new(tab_bar.wrap())
            .width(Length::Fill)
            .style(|t| Style {
                text_color: None,
                background: Some(Background::Color(BACKGROUND_COLOR)),
                border: Default::default(),
                shadow: Default::default(),
                snap: false,
            });

        let main_window = if self.tabs.len() == 0 {
            text!("This tab view is empty").into()
        } else {
            let selected = &self.tabs[self.selected];
            selected.view().map(move |action| Message::TabAction {
                pane,
                tab_index: self.selected,
                action,
            })
        };
        let main_window: Element<_, _, _> = container(main_window)
            .padding(PANE_RADIUS.max(5.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| {
                let mut radius = Radius::default();
                if right {
                    radius.top_right = PANE_RADIUS;
                }
                if below && right {
                    radius.bottom_right = PANE_RADIUS;
                }
                if below && left {
                    radius.bottom_left = PANE_RADIUS;
                }
                if left && self.selected != 0 {
                    radius.top_left = PANE_RADIUS;
                }
                container::Style {
                    text_color: None,
                    background: Some(Background::Color(Color::WHITE)),
                    border: Border::default().rounded(radius),
                    shadow: Default::default(),
                    snap: false,
                }
            })
            .into();

        let overlay = match &self.hover_preview {
            Some(PaneDragKind::Split { kind }) => {
                let active_inner = iced::widget::container(text![""])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|t| {
                        bordered_box(t)
                            .background(Background::Color(Color::BLACK.scale_alpha(0.1)))
                            .border(Border {
                                color: Color::BLACK.scale_alpha(0.5),
                                width: 4.0,
                                radius: Radius::new(10.0),
                            })
                    });
                let active = iced::widget::container(active_inner)
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1))
                    .padding(8.0);
                let filler = iced::widget::container(text!(""))
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1));
                let overlay_content: Element<_, _, _> = match kind {
                    SplitKind::Top => column![active, filler].into(),
                    SplitKind::Bottom => column![filler, active].into(),
                    SplitKind::Left => row![active, filler].into(),
                    SplitKind::Right => row![filler, active].into(),
                    &SplitKind::Center => row![active].into(),
                };
                Some(overlay_content)
            }
            _ => None,
        };

        let main_window = match overlay {
            Some(overlay) => stack![main_window, overlay].into(),
            None => main_window,
        };

        let window = container(column![tab_bar, main_window])
            .id(self.id.clone())
            .style(|t| container::Style {
                text_color: None,
                background: Some(Background::Color(BACKGROUND_COLOR)),
                border: Default::default(),
                shadow: Default::default(),
                snap: false,
            });

        window.into()
    }
}
