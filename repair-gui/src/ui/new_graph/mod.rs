use crate::controls::{WidgetGraph, WidgetGraphAction, WidgetGraphState};
use crate::{GlobalAction, SharedState};
use glam::Vec2;
use iced::widget::{button, text};
use iced::{Element, Point};
use tabbed_workspace::GlobalisedMessage;

#[derive(Clone)]
pub enum NewGraphMessage {
    Test,
    Boom,
    GraphMessage(WidgetGraphAction),
}

#[derive(Clone)]
pub struct NewGraph {
    graph_state: WidgetGraphState<&'static str>,
}

impl NewGraph {
    pub fn new() -> Self {
        let mut graph_state = WidgetGraphState::new();
        graph_state.add_node("asdf", Point::new(50.0, 50.0));
        graph_state.add_node("jklö", Point::new(300.0, 60.0));
        graph_state.add_node("yay", Point::new(80.0, 200.0));
        graph_state.add_node("nay", Point::new(120.0, 150.0));

        graph_state.add_connection("asdf", "yay");
        graph_state.add_connection("asdf", "nay");
        graph_state.add_connection("jklö", "yay");

        Self { graph_state }
    }

    pub fn update(&mut self, shared_state: &mut SharedState, message: NewGraphMessage) {
        match message {
            NewGraphMessage::Test => {
                println!("test")
            }
            NewGraphMessage::Boom => {
                println!("boom")
            }
            NewGraphMessage::GraphMessage(action) => self.graph_state.update(action),
        }
    }

    pub fn view(
        &self,
        shared_state: &SharedState,
    ) -> Element<'_, GlobalisedMessage<NewGraphMessage, GlobalAction>> {
        let mut graph = WidgetGraph::new(&self.graph_state, NewGraphMessage::GraphMessage);

        graph.add_child("asdf", text("asdf"));
        graph.add_child("jklö", button("jklö").on_press(NewGraphMessage::Boom));
        graph.add_child("yay", button("yay").on_press(NewGraphMessage::Test));
        graph.add_child("nay", button("nay"));

        let element: Element<_> = graph.into();
        element.map(|a| GlobalisedMessage::Local(a))
    }
}
