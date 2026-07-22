mod layout;
mod task_node;
mod window_builder;

use crate::ui::call_details::CallDetails;
use crate::ui::repair_graph::window_builder::{WindowMessage, WindowState};
use crate::{GlobalAction, SharedState, TabWindow};
use iced::widget::{Column, Row, Stack, container, row, scrollable, space, text};
use iced::{Color, Element, Length, Padding};
pub use layout::*;
use repair_lib::repair_graph::RepairGraphNode;
use repair_lib::task_graph::ParameterValue;
use tabbed_workspace::GlobalisedMessage;

#[derive(Clone)]
pub enum RepairGraphMessage {
    ModelNodeMessage {
        model_index: usize,
        message: WindowMessage<ModelMessage>,
    },
    TaskNodeMessage {
        model_index: usize,
        task_index: usize,
        message: WindowMessage<TaskMessage>,
    },
}

#[derive(Clone)]
pub enum ModelMessage {}

#[derive(Clone)]
pub enum TaskMessage {
    SetValue {
        parameter_index: usize,
        value: ParameterValue,
    },
    Run,
}

#[derive(Clone)]
pub struct RepairGraphUITab {}

impl RepairGraphUITab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, shared_state: &mut SharedState, message: RepairGraphMessage) {
        match message {
            RepairGraphMessage::ModelNodeMessage {
                model_index,
                message,
            } => match message {
                WindowMessage::Internal(message) => {
                    let position = &shared_state.repair_graph_layout.node_to_position[model_index]
                        .model_position;
                    shared_state.repair_graph_layout.rows[position.row].entries[position.column]
                        .as_mut()
                        .unwrap()
                        .window_state
                        .update(message);
                }
                WindowMessage::ContentMessage(message) => {
                    self.update_model_node(message, shared_state, model_index)
                }
            },
            RepairGraphMessage::TaskNodeMessage {
                model_index,
                task_index,
                message,
            } => match message {
                WindowMessage::Internal(message) => {
                    let position = &shared_state.repair_graph_layout.node_to_position[model_index]
                        .task_positions[task_index];
                    shared_state.repair_graph_layout.rows[position.row].entries[position.column]
                        .as_mut()
                        .unwrap()
                        .window_state
                        .update(message);
                }
                WindowMessage::ContentMessage(message) => {
                    self.update_task_node(message, shared_state, model_index, task_index)
                }
            },
        }
    }

    fn update_model_node(
        &mut self,
        message: ModelMessage,
        shared_state: &mut SharedState,
        model_index: usize,
    ) {
        match message {}
    }

    fn update_task_node(
        &mut self,
        message: TaskMessage,
        shared_state: &mut SharedState,
        model_index: usize,
        task_index: usize,
    ) {
        let mut graph = shared_state.repair_problem.graph.lock().unwrap();
        match message {
            TaskMessage::SetValue {
                parameter_index,
                value,
            } => graph.nodes[model_index].tasks.tasks[task_index]
                .description
                .set_parameter_value(parameter_index, value),
            TaskMessage::Run => graph.run_task(model_index, task_index),
        }
    }

    pub fn view(
        &self,
        shared_state: &SharedState,
    ) -> Element<'_, GlobalisedMessage<RepairGraphMessage, crate::GlobalAction>> {
        let mut height: f32 = 0.0;

        let mut graph = shared_state.repair_problem.graph.lock().unwrap();

        let mut column = Column::new();
        for layout_row in &shared_state.repair_graph_layout.rows {
            let mut row = Row::new();

            let mut first = true;
            for entry in &layout_row.entries {
                if !first {
                    row = row.push(space().width(25));
                }
                first = false;
                let node = if let Some(layout) = entry {
                    let (model_index, task_index) = layout.node_index;
                    if let Some(task_index) = task_index {
                        task_node::task_node(
                            shared_state.repair_graph_layout.options.node_width,
                            model_index,
                            task_index,
                            &layout.window_state,
                            &graph.nodes[model_index].tasks.tasks[task_index],
                            &graph.tool_runner,
                        )
                    } else {
                        self.model_node(
                            shared_state.repair_graph_layout.options.node_width,
                            model_index,
                            &layout.window_state,
                            &graph.nodes[model_index],
                        )
                    }
                } else {
                    space().into()
                };
                row = row.push(
                    container(node).width(shared_state.repair_graph_layout.options.node_width),
                );
            }

            column = column.push(row);
            column = column.push(space().height(40));
        }

        let repair_graph = row![
            container(space()).width(Length::FillPortion(1)),
            column,
            container(space()).width(Length::FillPortion(1)),
        ];
        let scrollable: Element<_> = scrollable(repair_graph).width(Length::Fill).into();
        scrollable
    }

    fn model_node<'a>(
        &self,
        width: f32,
        model_index: usize,
        window_state: &WindowState,
        model: &RepairGraphNode,
    ) -> Element<'a, GlobalisedMessage<RepairGraphMessage, GlobalAction>> {
        let mut window_builder: window_builder::WindowBuilder<ModelMessage, GlobalAction> =
            window_builder::WindowBuilder::new(
                window_state,
                Color::from_rgb(1.0, 0.8, 0.75),
                width,
            );
        window_builder.add_header(format!("Model {model_index}"));

        let variable_count = model
            .model
            .variable_manager
            .variables
            .iter()
            .filter(|v| !v.is_constant())
            .count();
        let module_count = model.model.modules.len();
        let command_count: usize = model.model.modules.iter().map(|m| m.commands.len()).sum();
        let model_summary = format!(
            "{} variables, {} modules,  {} commands",
            variable_count, module_count, command_count,
        );
        let property_summary = format!("{} properties", model.properties.properties.len());

        window_builder.add_control(text!("{model_summary}").into());
        window_builder.add_control(text!("{property_summary}").into());

        let window = window_builder.finish();
        window.map(move |message| {
            message.map(move |message| RepairGraphMessage::ModelNodeMessage {
                model_index,
                message,
            })
        })
    }

    fn place_node<'a>(
        &self,
        x: f32,
        y: f32,
        content: Element<'a, RepairGraphMessage>,
    ) -> Element<'a, RepairGraphMessage> {
        container(content)
            .padding(Padding::default().left(x).top(y))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl From<RepairGraphUITab> for TabWindow {
    fn from(value: RepairGraphUITab) -> Self {
        TabWindow::RepairGraph(value)
    }
}
