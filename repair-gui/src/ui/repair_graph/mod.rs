mod layout;
mod task_node;
mod window_builder;

use crate::SharedState;
use crate::ui::repair_graph::window_builder::{WindowMessage, WindowState};
use iced::widget::{Stack, container, row, scrollable, space, text};
use iced::{Color, Element, Length, Padding};
pub use layout::*;
use repair_lib::repair_graph::RepairGraphNode;
use repair_lib::task_graph::ParameterValue;

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
                WindowMessage::Internal(message) => shared_state.repair_graph_layout.layout
                    [model_index]
                    .model_position
                    .window_state
                    .update(message),
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
                    shared_state.repair_graph_layout.layout[model_index].task_positions[task_index]
                        .window_state
                        .update(message)
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

    pub fn view<'a>(&'a self, shared_state: &SharedState) -> Element<'a, RepairGraphMessage> {
        let width = shared_state.repair_graph_layout.options.width;
        let mut height: f32 = 0.0;

        let mut stack: Stack<RepairGraphMessage> = Stack::new();

        let x_offset = shared_state.repair_graph_layout.options.width * 0.5;
        let node_width = shared_state.repair_graph_layout.options.node_width;
        let node_height = shared_state.repair_graph_layout.options.node_height;
        let mut graph = shared_state.repair_problem.graph.lock().unwrap();
        for (model_index, model) in graph.nodes.iter().enumerate() {
            if let Some(position) = shared_state.repair_graph_layout.model_position(model_index) {
                let model_node = self.model_node(
                    node_width,
                    node_height,
                    model_index,
                    &position.window_state,
                    model,
                );
                stack = stack.push(self.place_node(
                    position.position.x - node_width * 0.5 + x_offset,
                    position.position.y,
                    model_node,
                ))
            }
            for (task_index, task) in model.tasks.tasks.iter().enumerate() {
                if let Some(position) = shared_state
                    .repair_graph_layout
                    .task_position(model_index, task_index)
                {
                    let task_node = task_node::task_node(
                        node_width,
                        node_height,
                        model_index,
                        task_index,
                        &position.window_state,
                        task,
                        &graph.tool_runner,
                    );
                    stack = stack.push(self.place_node(
                        position.position.x - node_width * 0.5 + x_offset,
                        position.position.y,
                        task_node,
                    ));
                    height = height.max(position.position.y + node_height + 100.0);
                }
            }
        }

        let base = container(stack)
            .width(shared_state.repair_graph_layout.options.width)
            .height(height)
            .clip(true);

        let repair_graph = row![
            container(space()).width(Length::FillPortion(1)),
            base,
            container(space()).width(Length::FillPortion(1)),
        ];
        scrollable(repair_graph).width(Length::Fill).into()
    }

    fn model_node<'a>(
        &self,
        width: f32,
        height: f32,
        model_index: usize,
        window_state: &WindowState,
        model: &RepairGraphNode,
    ) -> Element<'a, RepairGraphMessage> {
        let mut window_builder = window_builder::WindowBuilder::new(
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
        window.map(move |message| RepairGraphMessage::ModelNodeMessage {
            model_index,
            message,
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
