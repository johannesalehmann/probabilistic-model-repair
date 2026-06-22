use iced::futures::FutureExt;
use iced::widget::container::bordered_box;
use iced::widget::pane_grid::Pane;
use iced::widget::{Container, PaneGrid, Row, button, column, container, pane_grid, row, text};
use iced::{Element, Task};
use iced_core::Widget;
use iced_core::widget::Id;
use std::collections::HashMap;

pub struct TabbedWorkspace<W: Window> {
    selected_window: pane_grid::Pane,
    pane_grid_state: pane_grid::State<TabView<W>>,
    id_to_pane: HashMap<Id, Pane>,
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
        }
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
            Message::Split { pane, axis } => {
                let (new_pane, new_id) = TabView::new();
                let (pane_id, _) = self.pane_grid_state.split(axis, pane, new_pane).unwrap();
                self.id_to_pane.insert(new_id, pane_id);
            }
            Message::DropTab {
                old_pane,
                tab_index,
                location,
                bounds,
            } => {
                return iced_drop::zones_on_point(
                    move |zones| Message::HandleDropZones {
                        old_pane,
                        tab_index,
                        zones,
                    },
                    location,
                    None,
                    None,
                );
            }
            Message::HandleDropZones {
                old_pane,
                tab_index,
                zones,
            } => {
                if zones.len() == 1 {
                    let (id, rect) = &zones[0];
                    if let Some(pane) = self.id_to_pane.get(id) {
                        let old_pane = self.pane_grid_state.get_mut(old_pane).unwrap();
                        let tab = old_pane.tabs.remove(tab_index);
                        if old_pane.selected >= tab_index && tab_index > 0 {
                            old_pane.selected -= 1;
                        }

                        let new_pane = self.pane_grid_state.get_mut(*pane).unwrap();
                        new_pane.selected = new_pane.tabs.len();
                        new_pane.tabs.push(tab);
                    } else {
                        println!(
                            "Could not match id to zone (id: {:?}, rect: {:?})",
                            id, rect
                        );
                    }
                } else if zones.len() > 1 {
                    println!("Multiple drop zones: {:?}", zones);
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
    Split {
        pane: Pane,
        axis: pane_grid::Axis,
    },

    DropTab {
        old_pane: Pane,
        tab_index: usize,
        location: iced::Point,
        bounds: iced::Rectangle,
    },
    HandleDropZones {
        old_pane: Pane,
        tab_index: usize,
        zones: Vec<(iced::advanced::widget::Id, iced::Rectangle)>,
    },

    TabAction {
        pane: Pane,
        tab_index: usize,
        action: TabAction,
    },
}

struct TabView<W: Window> {
    tabs: Vec<W>,
    selected: usize,
    id: Id,
}

impl<W: Window> TabView<W> {
    pub fn new() -> (Self, Id) {
        let id = Id::unique();
        (
            Self {
                tabs: Vec::new(),
                selected: 0,
                id: id.clone(),
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
                    location,
                    bounds,
                })
                .on_click(Message::SelectTab {
                    pane,
                    tab: tab_index,
                });

            tab_bar = tab_bar.push(droppable);
        }
        tab_bar = tab_bar.push(button("|").on_press(Message::Split {
            pane,
            axis: pane_grid::Axis::Vertical,
        }));
        tab_bar = tab_bar.push(button("--").on_press(Message::Split {
            pane,
            axis: pane_grid::Axis::Horizontal,
        }));

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

        let window = container(column![tab_bar.wrap(), main_window]).id(self.id.clone());

        window.into()
    }
}
