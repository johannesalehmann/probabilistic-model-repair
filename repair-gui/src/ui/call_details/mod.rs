use crate::controls::TextBuilder;
use crate::{GlobalAction, SharedState, TabWindow};
use iced::Element;
use iced::widget::{Column, scrollable, space};
use repair_lib::tool_runner::LogDetails;
use tabbed_workspace::GlobalisedMessage;

#[derive(Clone)]
pub enum CallDetailsMessage {}

#[derive(Clone)]
pub struct CallDetails {
    pub call_id: (usize, usize),
}

impl CallDetails {
    pub fn new(call_id: (usize, usize)) -> Self {
        Self { call_id }
    }

    pub fn update(&mut self, shared_state: &mut SharedState, message: CallDetailsMessage) {
        match message {}
    }

    pub fn view(
        &self,
        shared_state: &SharedState,
    ) -> Element<'_, GlobalisedMessage<CallDetailsMessage, GlobalAction>> {
        let graph = shared_state.repair_problem.graph.lock().unwrap();
        let task = graph.tool_runner.entry(self.call_id);
        if let LogDetails::ToolCall {
            tool_name,
            arguments,
            output,
        } = &task.details
        {
            let mut column = Column::new();
            column = column.push(TextBuilder::new().with_bold("Call").build());

            let mut call_text = crate::controls::TextBuilder::new();
            call_text = call_text.with_typewriter(tool_name);
            for argument in arguments {
                if let Some(arg) = argument.to_str() {
                    call_text = call_text.with_typewriter(format!(" {arg}"));
                } else {
                    call_text = call_text.with_typewriter(" [argument is not a valid string]");
                }
            }
            column = column.push(call_text.build());

            if let Some(output) = output {
                column = column.push(space().height(30));

                column = column.push(TextBuilder::new().with_bold("Output").build());
                column = column.push(TextBuilder::new().with_typewriter(output).build());
            }
            scrollable(column).into()
        } else {
            unreachable!()
        }
    }
}

impl From<CallDetails> for TabWindow {
    fn from(value: CallDetails) -> Self {
        TabWindow::CallDetails(value)
    }
}
