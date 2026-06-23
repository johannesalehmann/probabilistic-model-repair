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
use iced_drop::widget::droppable::Droppable;
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

const PREVIEW_BORDER_COLOR: Color = Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};
const PREVIEW_BACKGROUND_COLOR: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.3,
};
const PREVIEW_BORDER_WIDTH: f32 = 0.0;
const PREVIEW_BORDER_RADIUS: f32 = 10.0;
const PREVIEW_INSET: f32 = 8.0;

const TAB_BAR_DROP_ZONE_OVERHANG: f32 = 16.0;

pub struct TabbedWorkspace<W: Window> {
    selected_window: pane_grid::Pane,
    pane_grid_state: pane_grid::State<TabView<W>>,
    id_to_pane: HashMap<Id, DropLocation>,
    hover_preview_pane: Option<Pane>,
    hidden_tab: Option<Pane>,
}

#[derive(Eq, PartialEq, Hash)]
pub struct DropLocation {
    pane: Pane,
    component: DropComponent,
}

#[derive(Eq, PartialEq, Hash)]
pub enum DropComponent {
    TabBar { index: usize },
    MainWindow,
}

impl<W: Window> TabbedWorkspace<W> {
    pub fn new() -> Self {
        let (default_window, default_id) = TabView::new();
        let (pane_grid_state, selected_window) = pane_grid::State::new(default_window);
        let mut res = Self {
            pane_grid_state,
            selected_window,
            id_to_pane: HashMap::new(),
            hover_preview_pane: None,
            hidden_tab: None,
        };
        res.add_drop_ids(selected_window, default_id);
        res
    }

    fn add_drop_ids(&mut self, pane: Pane, new_ids: Vec<(Id, DropComponent)>) {
        for (id, component) in new_ids {
            self.id_to_pane.insert(id, DropLocation { pane, component });
        }
    }

    fn unhide_tab(&mut self) {
        let pane_id = self.hidden_tab.unwrap();
        let pane = self.pane_grid_state.get_mut(pane_id).unwrap();
        pane.hidden_tab = None;
        self.hidden_tab = None;
    }

    fn take_tab(&mut self, old_pane_id: Pane, old_index: usize, new_pane_id: Pane) -> W {
        let old_pane = self.pane_grid_state.get_mut(old_pane_id).unwrap();
        let tab = old_pane.tabs.remove(old_index);

        if old_pane.selected >= old_index && old_pane.selected > 0 {
            old_pane.selected -= 1;
        }

        if old_pane_id != new_pane_id && old_pane.tabs.is_empty() {
            self.close_pane(old_pane_id);
        }

        tab
    }

    fn close_pane(&mut self, pane_id: Pane) {
        let result = self.pane_grid_state.close(pane_id);
        if let Some((tab, _)) = result {
            self.clear_drop_ids(tab)
        }
    }

    fn clear_drop_ids(&mut self, tab_group: TabView<W>) {
        self.id_to_pane.remove(&tab_group.id).unwrap();
        for id in tab_group.tab_bar_left_zone_id {
            self.id_to_pane.remove(&id);
        }
        for id in tab_group.tab_bar_right_zone_id {
            self.id_to_pane.remove(&id);
        }
        for id in tab_group.tab_bar_spacer_zone_id {
            self.id_to_pane.remove(&id);
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
            if let Some(drop_location) = self.id_to_pane.get(id) {
                let kind = match drop_location.component {
                    DropComponent::TabBar { index } => PaneDragKind::Tab { index },
                    DropComponent::MainWindow => {
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
                        PaneDragKind::Split { kind }
                    }
                };

                return Some((drop_location.pane.clone(), kind));
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
                    self.close_pane(pane_id);
                }
            }

            Message::Drag {
                old_pane_id,
                old_index,
                cursor,
                bounds,
            } => {
                if self.hidden_tab.is_none() {
                    let pane = self.pane_grid_state.get_mut(old_pane_id).unwrap();
                    pane.hidden_tab = Some(old_index);
                    pane.hover_preview = Some(PaneDragKind::Tab { index: old_index });
                    self.hidden_tab = Some(old_pane_id);
                }

                return iced_drop::zones_on_point(
                    move |zones| Message::HandleHoverZones { zones, cursor },
                    cursor,
                    None,
                    None,
                );
            }

            Message::HandleHoverZones { cursor, zones } => {
                self.clear_preview();
                if let Some((pane, drop_kind)) = self.get_hover_location(cursor, &zones[..]) {
                    let new_pane = self.pane_grid_state.get_mut(pane).unwrap();
                    new_pane.hover_preview = Some(drop_kind);
                    self.hover_preview_pane = Some(pane);
                }
            }

            Message::DropTab {
                cursor,
                bounds,
                old_pane_id,
                old_index,
            } => {
                return iced_drop::zones_on_point(
                    move |zones| Message::HandleDropZones {
                        zones,
                        cursor,
                        old_pane_id,
                        old_index,
                    },
                    cursor,
                    None,
                    None,
                );
            }
            Message::HandleDropZones {
                cursor,
                zones,
                old_pane_id,
                old_index,
            } => {
                self.clear_preview();
                self.unhide_tab();
                if let Some((pane, drop_kind)) = self.get_hover_location(cursor, &zones[..]) {
                    let tab = self.take_tab(old_pane_id, old_index, pane);
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
                            let (mut new_tab_view, new_ids) = TabView::new();
                            new_tab_view.tabs.push(tab);
                            let (split_result, _) = self
                                .pane_grid_state
                                .split(axis, pane, new_tab_view)
                                .unwrap();
                            self.add_drop_ids(split_result, new_ids);
                            if kind == SplitKind::Top || kind == SplitKind::Left {
                                self.pane_grid_state.swap(pane, split_result);
                            }
                        }
                    }
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
                println!("Cancelling drag");
                self.clear_preview();
                self.unhide_tab();
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

    Drag {
        old_pane_id: Pane,
        old_index: usize,
        cursor: iced::Point,
        bounds: iced::Rectangle,
    },
    HandleHoverZones {
        cursor: iced::Point,
        zones: Vec<(Id, iced::Rectangle)>,
    },
    DropTab {
        old_pane_id: Pane,
        old_index: usize,
        cursor: iced::Point,
        bounds: iced::Rectangle,
    },
    HandleDropZones {
        old_pane_id: Pane,
        old_index: usize,
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
    tab_bar_left_zone_id: Vec<Id>,
    tab_bar_right_zone_id: Vec<Id>,
    tab_bar_spacer_zone_id: Vec<Id>,
    hidden_tab: Option<usize>,
}

impl<W: Window> TabView<W> {
    pub fn new() -> (Self, Vec<(Id, DropComponent)>) {
        let id = Id::unique();
        let tab_bar_left_zone_id: Vec<_> = (0..256).map(|_| Id::unique()).collect();
        let tab_bar_right_zone_id: Vec<_> = (0..256).map(|_| Id::unique()).collect();
        let tab_bar_spacer_zone_id: Vec<_> = (0..256).map(|_| Id::unique()).collect();

        let mut id_to_component = Vec::new();
        id_to_component.push((id.clone(), DropComponent::MainWindow));
        for (index, id) in tab_bar_left_zone_id.iter().enumerate() {
            id_to_component.push((id.clone(), DropComponent::TabBar { index }));
        }
        for (index, id) in tab_bar_right_zone_id.iter().enumerate() {
            id_to_component.push((id.clone(), DropComponent::TabBar { index }));
        }
        for (index, id) in tab_bar_spacer_zone_id.iter().enumerate() {
            id_to_component.push((id.clone(), DropComponent::TabBar { index }));
        }

        (
            Self {
                tabs: Vec::new(),
                selected: 0,
                id: id.clone(),
                hover_preview: None,
                tab_bar_left_zone_id,
                tab_bar_right_zone_id,
                tab_bar_spacer_zone_id,
                hidden_tab: None,
            },
            id_to_component,
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
        let mut tab_bar = Row::new().height(33).width(Length::Fill);
        if !above {
            tab_bar = tab_bar.padding(Padding::default().top(PANE_SPACING))
        }
        let preview_index = self.hover_preview.as_ref().and_then(|p| match p {
            PaneDragKind::Split {
                kind: SplitKind::Center,
            } => Some(self.tabs.len()),
            PaneDragKind::Split { .. } => None,
            PaneDragKind::Tab { index } => Some(*index),
        });
        let mut insertion_index = 0;
        let mut prev_hidden = false;
        for (tab_index, tab) in self.tabs.iter().enumerate() {
            let hidden = Some(tab_index) == self.hidden_tab;
            if let Some(id) = self.tab_bar_spacer_zone_id.get(insertion_index) {
                if Some(insertion_index) == preview_index && !prev_hidden {
                    let left_spacing = if tab_index == 0 { 0.001 } else { 5.0 };
                    let right_spacing = if hidden { 4.0 } else { 5.0 };
                    tab_bar = tab_bar.push(
                        Self::view_preview_header(left_spacing, right_spacing).id(id.clone()),
                    );
                } else {
                    let spacer_width = if tab_index == 0 || prev_hidden {
                        0.001
                    } else if hidden {
                        4.0
                    } else {
                        5.0
                    };
                    tab_bar = tab_bar.push(
                        container(Space::new())
                            .width(spacer_width)
                            .height(Length::Fill)
                            .id(id.clone()),
                    );
                }
            }

            let header = self.view_tab_header(pane, tab_index, insertion_index, tab, !hidden);
            let smaller_container = container(header)
                .width(1.0)
                .padding(Padding::default().right(-110.0))
                .clip(hidden);
            tab_bar = tab_bar.push(smaller_container);
            if !hidden {
                tab_bar = tab_bar.push(container(Space::new()).width(109.0));
            } else {
                tab_bar = tab_bar.push(container(Space::new()).width(0.0));
            }
            if !hidden {
                insertion_index += 1;
            }
            prev_hidden = hidden;
        }
        if Some(insertion_index) == preview_index && !prev_hidden {
            if let Some(id) = self.tab_bar_spacer_zone_id.get(insertion_index) {
                let left_spacing = if self.tabs.len() == 0 { 0.001 } else { 5.0 };
                tab_bar = tab_bar.push(Self::view_preview_header(left_spacing, 0.0).id(id.clone()));
            }
        }

        if let Some(id) = self.tab_bar_left_zone_id.get(insertion_index) {
            tab_bar = tab_bar.push(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .id(id.clone()),
            )
        }

        let tab_bar = Container::new(tab_bar)
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
            Some(kind) => {
                let active_inner = iced::widget::container(text![""])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|t| {
                        bordered_box(t)
                            .background(Background::Color(PREVIEW_BACKGROUND_COLOR))
                            .border(Border {
                                color: PREVIEW_BORDER_COLOR,
                                width: PREVIEW_BORDER_WIDTH,
                                radius: Radius::new(PREVIEW_BORDER_RADIUS),
                            })
                    });
                let active = iced::widget::container(active_inner)
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1))
                    .padding(PREVIEW_INSET);
                let filler = iced::widget::container(text!(""))
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1));
                let overlay_content: Element<_, _, _> = match kind {
                    PaneDragKind::Split {
                        kind: SplitKind::Top,
                    } => column![active, filler].into(),
                    PaneDragKind::Split {
                        kind: SplitKind::Bottom,
                    } => column![filler, active].into(),
                    PaneDragKind::Split {
                        kind: SplitKind::Left,
                    } => row![active, filler].into(),
                    PaneDragKind::Split {
                        kind: SplitKind::Right,
                    } => row![filler, active].into(),
                    PaneDragKind::Split {
                        kind: SplitKind::Center,
                    } => row![active].into(),
                    PaneDragKind::Tab { .. } => row![active].into(),
                };
                Some(overlay_content)
            }
            _ => None,
        };

        let main_window = container(match overlay {
            Some(overlay) => stack![main_window, overlay].into(),
            None => main_window,
        })
        .id(self.id.clone());

        let window = container(column![tab_bar, main_window]).style(|t| container::Style {
            text_color: None,
            background: Some(Background::Color(BACKGROUND_COLOR)),
            border: Default::default(),
            shadow: Default::default(),
            snap: false,
        });

        window.into()
    }

    fn view_preview_header<'a>(
        left_spacing: f32,
        right_spacing: f32,
    ) -> Container<'a, Message<<W as Window>::TabAction>> {
        let border = Border {
            radius: Radius {
                top_left: PANE_RADIUS,
                top_right: PANE_RADIUS,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            ..Default::default()
        };
        let white = container(Space::new())
            .width(110.0)
            .height(Length::Fill)
            .style(move |t| container::Style {
                background: Some(Background::Color(Color::WHITE)),
                border: border.clone(),
                ..Default::default()
            });
        let tinted = container(Space::new())
            .width(110.0)
            .height(Length::Fill)
            .style(move |t| container::Style {
                background: Some(Background::Color(PREVIEW_BACKGROUND_COLOR)),
                border: border.clone(),
                ..Default::default()
            });
        let inner = stack![white, tinted];
        let preview_header = container(inner).padding(
            Padding::default()
                .left(left_spacing)
                .right(right_spacing)
                .bottom(-PREVIEW_INSET),
        );
        preview_header
    }

    fn view_tab_header<'a>(
        &'a self,
        pane: Pane,
        tab_index: usize,
        insertion_index: usize,
        tab: &W,
        with_ids: bool,
    ) -> Droppable<Message<<W as Window>::TabAction>> {
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
        let text =
            tooltip(text, tooltip_content, Position::Bottom).delay(Duration::from_secs_f64(0.2));
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
        .width(110);
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

        let id_hover = if !with_ids {
            row![]
        } else if insertion_index + 1 >= self.tab_bar_right_zone_id.len() {
            println!("Too many tabs. Dragging and dropping tabs may not work correctly");
            // TODO: Handle this properly (also above)
            row![]
        } else {
            let left_id = self.tab_bar_left_zone_id[insertion_index].clone();
            let right_id = self.tab_bar_right_zone_id[insertion_index + 1].clone();
            row![
                container(Space::new())
                    .id(left_id)
                    .height(Length::Fill)
                    .width(Length::FillPortion(1)),
                container(Space::new())
                    .id(right_id)
                    .height(Length::Fill)
                    .width(Length::FillPortion(1))
            ]
        };

        let with_zones = stack![header, id_hover];

        let droppable = iced_drop::droppable(with_zones)
            .on_drop(move |location, bounds| Message::DropTab {
                old_pane_id: pane,
                old_index: tab_index,
                cursor: location,
                bounds,
            })
            .on_drag(move |location, bounds| Message::Drag {
                old_pane_id: pane,
                old_index: tab_index,
                cursor: location,
                bounds,
            })
            .on_cancel(Message::CancelDrag)
            .on_click(Message::SelectTab {
                pane,
                tab: tab_index,
            })
            .drag_hide(true);
        droppable
    }
}
