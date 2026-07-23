use crate::controls::{WidgetGraph, WidgetGraphState};
use crate::{GlobalAction, SharedState};
use glam::Vec2;
use iced::Element;
use iced::widget::button;
use tabbed_workspace::GlobalisedMessage;

#[derive(Clone)]
pub enum NewGraphMessage {}

#[derive(Clone)]
pub struct NewGraph {
    graph_state: WidgetGraphState<&'static str>,
}

impl NewGraph {
    pub fn new() -> Self {
        let mut graph_state = WidgetGraphState::new();
        graph_state.add_node("asdf", Vec2::new(50.0, 50.0));
        graph_state.add_node("jklö", Vec2::new(300.0, 60.0));
        graph_state.add_node("yay", Vec2::new(80.0, 200.0));
        graph_state.add_node("nay", Vec2::new(120.0, 150.0));

        graph_state.add_connection("asdf", "yay");
        graph_state.add_connection("asdf", "nay");
        graph_state.add_connection("jklö", "yay");

        Self { graph_state }
    }

    pub fn update(&mut self, shared_state: &mut SharedState, message: NewGraphMessage) {
        match message {}
    }

    pub fn view(
        &self,
        shared_state: &SharedState,
    ) -> Element<'_, GlobalisedMessage<NewGraphMessage, GlobalAction>> {
        let mut graph = WidgetGraph::new(&self.graph_state);

        graph.add_child("asdf", button("asdf"));
        graph.add_child("jklö", button("jklö"));
        graph.add_child("yay", button("yay"));
        graph.add_child("nay", button("nay"));

        graph.into()
    }
}
