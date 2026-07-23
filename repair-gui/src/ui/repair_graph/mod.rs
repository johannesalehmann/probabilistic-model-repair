mod layout;
mod task_node;
mod window_builder;

use crate::controls::{WidgetGraph, WidgetGraphAction, WidgetGraphState};
use crate::ui::repair_graph::window_builder::{WindowMessage, WindowState};
use crate::{GlobalAction, SharedState, TabWindow};
use iced::widget::{Column, Row, Stack, container, row, scrollable, space, text};
use iced::{Color, Element, Length, Padding, Point, Vector};
use repair_lib::repair_graph::{RepairGraph, RepairGraphNode};
use repair_lib::task_graph::ParameterValue;
use tabbed_workspace::GlobalisedMessage;

pub const NODE_WIDTH: f32 = 150.0;
pub const GRAPH_WIDTH: f32 = 900.0;

pub struct RepairGraphLayout {
    layout: WidgetGraphState<(usize, Option<usize>)>,
    window_states: Vec<(WindowState, Vec<WindowState>)>,
}

impl RepairGraphLayout {
    pub fn new() -> Self {
        Self {
            layout: WidgetGraphState::new().with_drag_limit_x(GRAPH_WIDTH),
            window_states: Vec::new(),
        }
    }

    pub fn update_layout(&mut self, graph: &RepairGraph) {
        let v_size = 100.0;
        let mut new_dependencies = Vec::new();

        for (model_index, model) in graph.nodes.iter().enumerate() {
            if self.window_states.len() <= model_index {
                self.window_states
                    .push((WindowState::with_expanded_sections(4), Vec::new()));
            }

            let model_position = if let Some(node) = self.layout.node((model_index, None)) {
                node.position
            } else {
                let position = if let Some(parent) = &model.parent {
                    new_dependencies.push((
                        (parent.model_index, Some(parent.task_index)),
                        (model_index, None),
                    ));
                    self.layout
                        .node((parent.model_index, Some(parent.task_index)))
                        .map(|n| n.position)
                        .unwrap_or(Point::new(GRAPH_WIDTH * 0.5, 0.0))
                        + Vector::new(0.0, v_size)
                } else {
                    Point::new(GRAPH_WIDTH * 0.5, 0.0)
                };

                self.layout
                    .add_node((model_index, None), position, NODE_WIDTH);
                position
            };
            for (task_index, task) in model.tasks.tasks.iter().enumerate() {
                if self.window_states[model_index].1.len() <= task_index {
                    self.window_states[model_index]
                        .1
                        .push(WindowState::with_expanded_sections(4));
                }
                if self.layout.node((model_index, Some(task_index))).is_none() {
                    for &dependency in &task.dependencies {
                        new_dependencies.push((
                            (model_index, Some(dependency)),
                            (model_index, Some(task_index)),
                        ));
                    }
                    if task.dependencies.len() == 0 {
                        new_dependencies
                            .push(((model_index, None), (model_index, Some(task_index))));
                    }

                    let position = if task.dependencies.len() == 0 {
                        model_position + Vector::new(0.0, v_size)
                    } else {
                        let mut x_sum = 0.0;
                        let mut y_max = 0.0f32;
                        let mut counter = 0;
                        task.dependencies
                            .iter()
                            .map(|dep| self.layout.node((model_index, Some(*dep))))
                            .filter(|node| node.is_some())
                            .map(|node| (node.unwrap()))
                            .for_each(|node| {
                                x_sum += node.position.x;
                                y_max = y_max.max(node.position.y);
                                counter += 1;
                            });

                        if counter > 0 {
                            Point::new(x_sum / counter as f32, y_max + v_size)
                        } else {
                            model_position + Vector::new(0.0, v_size)
                        }
                    };

                    self.layout
                        .add_node((model_index, Some(task_index)), position, NODE_WIDTH);
                }
            }
        }
        for (from, to) in new_dependencies {
            self.layout.add_connection(from, to);
        }
    }
}

#[derive(Clone)]
pub enum RepairGraphMessage {
    GraphAction(WidgetGraphAction),
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
                    shared_state.repair_graph_layout.window_states[model_index]
                        .0
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
                    shared_state.repair_graph_layout.window_states[model_index].1[task_index]
                        .update(message);
                }
                WindowMessage::ContentMessage(message) => {
                    self.update_task_node(message, shared_state, model_index, task_index)
                }
            },
            RepairGraphMessage::GraphAction(action) => {
                shared_state.repair_graph_layout.layout.update(action)
            }
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

    pub fn view<'a>(
        &'a self,
        shared_state: &'a SharedState,
    ) -> Element<'a, GlobalisedMessage<RepairGraphMessage, crate::GlobalAction>> {
        let graph = shared_state.repair_problem.graph.lock().unwrap();

        let mut widget_graph: WidgetGraph<
            _,
            _,
            GlobalisedMessage<RepairGraphMessage, GlobalAction>,
            _,
            _,
        > = WidgetGraph::new(&shared_state.repair_graph_layout.layout, |msg| {
            GlobalisedMessage::Local(RepairGraphMessage::GraphAction(msg))
        })
        .width(GRAPH_WIDTH);

        for (model_index, model) in graph.nodes.iter().enumerate() {
            if shared_state
                .repair_graph_layout
                .layout
                .node((model_index, None))
                .is_some()
            {
                widget_graph.add_child(
                    (model_index, None),
                    self.model_node(
                        NODE_WIDTH,
                        model_index,
                        &shared_state.repair_graph_layout.window_states[model_index].0,
                        &graph.nodes[model_index],
                    ),
                );
            }
            for (task_index, task) in model.tasks.tasks.iter().enumerate() {
                if shared_state
                    .repair_graph_layout
                    .layout
                    .node((model_index, Some(task_index)))
                    .is_some()
                {
                    widget_graph.add_child(
                        (model_index, Some(task_index)),
                        task_node::task_node(
                            NODE_WIDTH,
                            model_index,
                            task_index,
                            &shared_state.repair_graph_layout.window_states[model_index].1
                                [task_index],
                            &graph.nodes[model_index].tasks.tasks[task_index],
                            &graph.tool_runner,
                        ),
                    );
                }
            }
        }

        let repair_graph = row![
            container(space()).width(Length::FillPortion(1)),
            widget_graph,
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
}

impl From<RepairGraphUITab> for TabWindow {
    fn from(value: RepairGraphUITab) -> Self {
        TabWindow::RepairGraph(value)
    }
}
