use iced::advanced::graphics::text::cosmic_text::skrifa::color::CompositeMode::Overlay;
use iced::application::UpdateFn;
use iced::futures::FutureExt;
use iced::widget::container::bordered_box;
use iced::widget::pane_grid::{Axis, Pane};
use iced::widget::{
    Container, PaneGrid, Row, button, column, container, pane_grid, row, stack, text,
};
use iced::{Element, Task};
use iced_core::border::Radius;
use iced_core::widget::Id;
use iced_core::{Background, Border, Color, Length, Point, Rectangle};
use std::collections::HashMap;

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
            Message::CloseTab { pane, tab } => {
                let pane = self.pane_grid_state.get_mut(pane).unwrap();
                pane.tabs.remove(tab);
                if pane.selected > 0 && pane.selected >= tab {
                    pane.selected -= 1;
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
                old_pane,
                tab_index,
                cursor,
                zones,
            } => {
                if let Some(old_preview) = self.hover_preview_pane {
                    self.pane_grid_state
                        .get_mut(old_preview)
                        .unwrap()
                        .hover_preview = None;
                }
                if let Some((pane, drop_kind)) = self.get_hover_location(cursor, &zones[..]) {
                    let old_pane = self.pane_grid_state.get_mut(old_pane).unwrap();
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
                }
            }

            Message::HandleHoverZones {
                old_pane,
                tab_index,
                cursor,
                zones,
            } => {
                if let Some(old_preview) = self.hover_preview_pane {
                    self.pane_grid_state
                        .get_mut(old_preview)
                        .unwrap()
                        .hover_preview = None;
                }
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
        }
        Task::none()
    }

    pub fn view<'a, Msg: 'a, F: 'a + Clone + Fn(Message<W::TabAction>) -> Msg>(
        &'a self,
        emit_message: F,
    ) -> Element<'a, Msg> {
        Element::map(
            PaneGrid::new(&self.pane_grid_state, |id, pane, maximised| {
                pane_grid::Content::new(pane.view(id))
            })
            .on_resize(3, Message::PaneGridResized)
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

    pub fn view<'a>(&'a self, pane: Pane) -> Element<'a, Message<W::TabAction>> {
        let mut tab_bar = Row::new();
        for (tab_index, tab) in self.tabs.iter().enumerate() {
            let header = Container::new(row!(
                text!("{}", tab.title()),
                button("x").style(button::text).on_press(Message::CloseTab {
                    pane,
                    tab: tab_index
                })
            ))
            .style(bordered_box);

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
                .on_click(Message::SelectTab {
                    pane,
                    tab: tab_index,
                });
            tab_bar = tab_bar.push(droppable);
        }

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
            .width(Length::Fill)
            .height(Length::Fill)
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
                                color: Color::BLACK.scale_alpha(0.7),
                                width: 5.0,
                                radius: Radius::new(10.0),
                            })
                    });
                let active = iced::widget::container(active_inner)
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1))
                    .padding(16.0);
                let filler = iced::widget::container(text!(""))
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1))
                    .padding(16.0);
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

        let window = container(column![tab_bar.wrap(), main_window]).id(self.id.clone());

        window.into()
    }
}
