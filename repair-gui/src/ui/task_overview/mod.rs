use crate::SharedState;
use crate::controls::{Status, fadeout_box};
use iced::widget::{Column, Row, container, space};
use iced::{Element, Length};
use repair_lib::tool_runner::{LogDetails, StatusKind};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub enum TaskViewMessage {
    Open,
}

pub struct TaskViewTab {}

impl TaskViewTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, shared_state: &mut SharedState, message: TaskViewMessage) {
        match message {
            TaskViewMessage::Open => todo!(),
        }
    }

    pub fn view(&self, shared_state: &SharedState) -> Element<'_, TaskViewMessage> {
        let graph = shared_state.repair_problem.graph.lock().unwrap();
        let mut rows = Column::new();
        for &index in &graph.tool_runner.entries_in_order {
            let task = graph.tool_runner.entry(index);
            let mut row_entries = Row::new();
            let mut status_text = crate::controls::TextBuilder::single_line();
            let status = match &task.details {
                LogDetails::ToolCall { output, .. } => {
                    if output.is_some() {
                        Status::Success
                    } else {
                        Status::Running
                    }
                }
                LogDetails::Section {
                    name,
                    total_units,
                    completed_units,
                    final_status,
                } => match final_status {
                    None => Status::Running,
                    Some(status) => match status.kind {
                        StatusKind::Success => Status::Success,
                        StatusKind::Failure => Status::Failure,
                    },
                },
            };
            status_text = status_text.with_status(status);
            status_text = status_text
                .with_time(task.end_time.unwrap_or(Instant::now()) - task.start_time)
                .that_has_width(Length::Fixed(80.0));
            status_text = status_text.with_quad();
            status_text = status_text
                .with_typewriter(format!("#{}-{}", index.0, index.1))
                .that_has_width(Length::Fixed(70.0))
                .with_quad();
            row_entries = row_entries.push(status_text.build());

            match &task.details {
                LogDetails::ToolCall {
                    tool_name,
                    arguments,
                    output,
                } => {
                    let mut call_text = crate::controls::TextBuilder::single_line();
                    call_text = call_text.with_bold_typewriter(tool_name);
                    for argument in arguments {
                        if let Some(arg) = argument.to_str() {
                            call_text = call_text.with_typewriter(format!(" {arg}"));
                        } else {
                            call_text =
                                call_text.with_typewriter(" [argument is not a valid string]");
                        }
                    }
                    row_entries = row_entries.push(fadeout_box(call_text.build(), Length::Fill));
                    row_entries = row_entries.push(space().width(20));
                    if let Some(output) = output {
                        let mut output_text = crate::controls::TextBuilder::single_line();
                        output_text = output_text.with_bold("Output: ");
                        row_entries = row_entries.push(output_text.build());
                        let mut output_text = crate::controls::TextBuilder::single_line();
                        let first_line = output.lines().next();
                        if let Some(first_line) = first_line {
                            output_text = output_text.with_typewriter(first_line);
                        }
                        row_entries =
                            row_entries.push(fadeout_box(output_text.build(), Length::Fill));
                        let mut output_text = crate::controls::TextBuilder::single_line();
                        let lines = output.lines().count();
                        output_text = output_text.with_text("(");
                        output_text = output_text
                            .with_link(format!("{} lines", lines), TaskViewMessage::Open);
                        output_text = output_text.with_text(")");
                        row_entries = row_entries.push(output_text.build());
                    }
                }
                LogDetails::Section {
                    name,
                    total_units,
                    completed_units,
                    final_status,
                } => {
                    let mut text = crate::controls::TextBuilder::single_line();
                    text = text.with_bold(name);
                    if let Some(total_units) = total_units {
                        let completed_units = completed_units.unwrap_or(0);
                        text = text
                            .with_quad()
                            .with_x_of_y(completed_units as i64, *total_units as i64);
                    }
                    if let Some(final_status) = final_status {
                        if let Some(details) = &final_status.details {
                            text = text.with_quad().with_text(format!("Output: {details}"))
                        }
                    }

                    row_entries = row_entries.push(text.build());
                }
            }
            rows = rows.push(row_entries);
        }

        rows.into()
    }
}
