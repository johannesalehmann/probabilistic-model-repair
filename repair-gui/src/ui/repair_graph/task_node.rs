use crate::ui::repair_graph::window_builder::{SectionKind, WindowBuilder, WindowState};
use crate::ui::repair_graph::{RepairGraphMessage, TaskMessage, window_builder};
use iced::widget::text::Wrapping;
use iced::widget::tooltip::Position;
use iced::widget::{button, checkbox, column, container, row, space, text, tooltip};
use iced::{Color, Element, Length, Padding, font};
use repair_lib::task_graph::{
    ParameterDescription, ParameterType, ParameterValue, TaskDescription, TaskGraphNode, TaskStatus,
};
use repair_lib::tool_runner::{LogDetails, MainToolRunner, StatusKind};

pub fn task_node<'a>(
    width: f32,
    model_index: usize,
    task_index: usize,
    window_state: &WindowState,
    task: &TaskGraphNode,
    logs: &MainToolRunner,
) -> Element<'a, RepairGraphMessage> {
    let mut window_builder =
        window_builder::WindowBuilder::new(window_state, Color::from_rgb(0.8, 0.8, 1.0), width);
    window_builder.add_header(format!("{}", task.description.name()));

    match &task.status {
        TaskStatus::Ready => {
            parameter_section(&task.description, &mut window_builder, false);
            window_builder.add_call_to_action("Run!".to_string(), Some(TaskMessage::Run));
        }
        TaskStatus::Running { handle, start_time } => {
            parameter_section(&task.description, &mut window_builder, true);
            log_section((model_index, task_index), logs, &mut window_builder);
            window_builder.add_call_to_action("Running!".to_string(), None);
        }
        TaskStatus::Done { output, elapsed } => {
            parameter_section(&task.description, &mut window_builder, true);
            log_section((model_index, task_index), logs, &mut window_builder);
        }
    }

    let window = window_builder.finish();
    window.map(move |message| RepairGraphMessage::TaskNodeMessage {
        model_index,
        task_index,
        message,
    })
}

fn parameter_section(
    description: &Box<dyn TaskDescription>,
    window_builder: &mut WindowBuilder<TaskMessage>,
    force_collapse: bool,
) {
    let parameters = description.parameter_descriptions();
    if parameters.len() > 0 {
        window_builder.start_section(
            format!("{}", description.parameter_summary()),
            if force_collapse {
                SectionKind::forced_close()
            } else {
                SectionKind::togglable()
            },
        );

        for (parameter_index, parameter) in parameters.iter().enumerate() {
            let value = description.parameter_value(parameter_index);
            window_builder.add_control(parameter_control(parameter_index, parameter, &value));
        }

        window_builder.end_section();
    } else {
        window_builder.add_control(text!["no parameters"].into());
    }
}

fn parameter_control<'a>(
    parameter_index: usize,
    parameter: &ParameterDescription,
    value: &ParameterValue,
) -> Element<'a, TaskMessage> {
    match parameter.values {
        ParameterType::Integer { min, max } => {
            let value = value.int().unwrap();
            let control = row![
                text!["{}: ", parameter.name],
                button("-").on_press(TaskMessage::SetValue {
                    parameter_index,
                    value: ParameterValue::Integer(value - 1)
                }),
                text!["{value}"],
                button("+").on_press(TaskMessage::SetValue {
                    parameter_index,
                    value: ParameterValue::Integer(value + 1)
                })
            ];
            control.into()
        }
        ParameterType::Float { .. } => {
            panic!("Float parameters are not yet supported")
        }
        ParameterType::Boolean => {
            let value = value.bool().unwrap();
            checkbox(value)
                .on_toggle(move |val| TaskMessage::SetValue {
                    parameter_index,
                    value: ParameterValue::Boolean(val),
                })
                .label(parameter.name)
                .into()
        }
        ParameterType::Select { .. } => {
            panic!("Select parameters are not yet supported")
        }
    }
}

fn log_section(
    task_id: (usize, usize),
    logs: &MainToolRunner,
    window_builder: &mut WindowBuilder<TaskMessage>,
) {
    window_builder.start_section("Logs", SectionKind::forced_open());

    window_builder.add_control(log_list(task_id, logs));
    window_builder.end_section();
}

fn log_list<'a>(task_id: (usize, usize), logs: &MainToolRunner) -> Element<'a, TaskMessage> {
    let mut entries = Vec::new();
    for &entry_id in logs.entries_for_task(task_id) {
        let entry = logs.entry(entry_id);

        let status: Element<TaskMessage> = if entry.end_time.is_some() {
            if let LogDetails::Section {
                final_status: Some(final_status),
                ..
            } = &entry.details
            {
                let icon = match final_status.kind {
                    StatusKind::Success => text!("✓"),
                    StatusKind::Failure => text!("✗"),
                    StatusKind::Unknown => text!("?"),
                };
                if let Some(details) = &final_status.details {
                    tooltip(
                        icon,
                        container(text!["{details}"])
                            .style(|_| container::Style::default().background(Color::WHITE)),
                        Position::FollowCursor,
                    )
                    .into()
                } else {
                    icon.into()
                }
            } else {
                text!("✓").into()
            }
        } else {
            text!("⌛").into()
        };
        let status = container(status)
            .width(20)
            .padding(Padding::default().right(5.0));

        let main = match &entry.details {
            LogDetails::ToolCall {
                tool_name,
                arguments,
                output,
            } => {
                let call = [tool_name.as_str()]
                    .into_iter()
                    .chain(arguments.iter().map(|a| a.to_str().unwrap()))
                    .collect::<Vec<_>>()
                    .join(" ");
                row![
                    text!("Call "),
                    container(
                        text!("{call}")
                            .font(font::Font::MONOSPACE)
                            .size(14)
                            .wrapping(Wrapping::None)
                    )
                    .width(Length::Fill)
                    .padding(Padding::default().left(4.0))
                    .clip(true)
                ]
            }
            LogDetails::Section {
                name,
                total_units,
                completed_units,
                final_status,
            } => {
                let text = container(text!["{name}"].wrapping(Wrapping::None))
                    .width(Length::Fill)
                    .clip(true);
                let units: Element<_> = if let Some(total_units) = total_units {
                    let completed_units = completed_units.unwrap_or(0);
                    text![" ({completed_units}/{total_units})"].into()
                } else {
                    space().into()
                };
                row![text, units]
            }
        };

        let elapsed = match entry.end_time {
            None => entry.start_time.elapsed(),
            Some(end_time) => end_time - entry.start_time,
        };
        let seconds = elapsed.as_secs();
        let minutes = seconds / 60;
        let hours = minutes / 60;
        let days = hours / 24;

        let seconds = seconds % 60;
        let minutes = minutes % 60;
        let hours = hours % 24;

        let mut time_components = Vec::new();
        if days > 0 {
            time_components.push(format!("{}d", days));
        }
        if hours > 0 {
            time_components.push(format!("{}h", hours));
        }
        if minutes > 0 {
            time_components.push(format!("{}m", minutes));
        }
        time_components.push(format!("{}s", seconds));

        let time = time_components.join(" ");
        let time = container(text!["{time}"]).padding(Padding::default().left(4.0));

        let entry: Element<_> = row![status, main, time].into();
        entries.push(entry);
    }
    column(entries.into_iter()).into()
}
