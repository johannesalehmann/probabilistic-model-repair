use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{
    Modifications, ModifiedTaskDependencies, Output, OutputsOfDependencies, ParameterDescription,
    ParameterType, ParameterValue, Task, TaskDescription, TaskOutput,
};
use crate::tool_runner::ToolRunner;
use std::any::Any;
use std::time::Duration;

pub struct DemoTaskDescription {
    delay_before: i64,
    delay_after: i64,
    call_prism: bool,
}

impl DemoTaskDescription {
    pub fn new() -> Self {
        Self {
            delay_before: 1,
            delay_after: 2,
            call_prism: false,
        }
    }
}

impl TaskDescription for DemoTaskDescription {
    fn name(&self) -> String {
        "DemoTask".into()
    }

    fn parameter_descriptions(&self) -> Vec<ParameterDescription> {
        vec![
            ParameterDescription::new(
                "Delay before",
                ParameterType::Integer {
                    min: Some(0),
                    max: None,
                },
            ),
            ParameterDescription::new("Run PRISM", ParameterType::Boolean),
            ParameterDescription::new(
                "Delay after",
                ParameterType::Integer {
                    min: Some(0),
                    max: None,
                },
            ),
        ]
    }

    fn parameter_value(&self, index: usize) -> ParameterValue {
        match index {
            0 => ParameterValue::Integer(self.delay_before),
            1 => ParameterValue::Boolean(self.call_prism),
            2 => ParameterValue::Integer(self.delay_after),
            _ => unreachable!(),
        }
    }

    fn set_parameter_value(&mut self, index: usize, value: ParameterValue) {
        match index {
            0 => self.delay_before = value.int().unwrap(),
            1 => self.call_prism = value.bool().unwrap(),
            2 => self.delay_after = value.int().unwrap(),
            _ => unreachable!(),
        }
    }

    fn parameter_summary(&self) -> String {
        let mut components = Vec::new();

        if self.delay_before > 0 {
            components.push(format!("Wait {} seconds", self.delay_before));
        }
        if self.call_prism {
            if components.is_empty() {
                components.push("Call prism".to_string());
            } else {
                components.push(", then call prism".to_string());
            }
        }
        if self.delay_after > 0 {
            if components.is_empty() {
                components.push("Wait".to_string());
            } else {
                components.push(", then wait".to_string());
            }
            components.push(format!(" {} seconds", self.delay_after))
        }
        if components.is_empty() {
            components.push("Do nothing".to_string())
        }

        components.join("")
    }

    fn create(&self) -> Box<dyn Task> {
        Box::new(DemoTask {
            delay_before: self.delay_before,
            delay_after: self.delay_after,
            call_prism: self.call_prism,
        })
    }
}

pub struct DemoTask {
    delay_before: i64,
    delay_after: i64,
    call_prism: bool,
}

#[async_trait::async_trait]
impl Task for DemoTask {
    async fn run(
        &mut self,
        _model: PrismModel,
        _properties: PropertyCollection,
        _inputs: OutputsOfDependencies,
        mut tool_runner: ToolRunner,
    ) -> TaskOutput {
        if self.delay_before > 0 {
            let section = tool_runner
                .start_section(format!("Waiting for {} seconds", self.delay_before))
                .await;
            tokio::time::sleep(Duration::from_secs(self.delay_before as u64)).await;
            section.finish_success().await;
        }

        if self.call_prism {
            tool_runner.run_tool("prism", &["-help"]).await;
        }

        if self.delay_after > 0 {
            let section = tool_runner
                .start_section_with_units(
                    format!("Waiting for {} seconds", self.delay_after),
                    self.delay_after as usize,
                )
                .await;
            for i in 0..self.delay_after {
                tokio::time::sleep(Duration::from_secs(1)).await;
                section.set_unit_count(i as usize + 1).await;
            }
            section
                .finish_failure_with_text("It failed because it felt like it!")
                .await;
        }

        let mut modifications = Modifications::new();

        for i in 0..2 {
            modifications.create_task(
                Box::new(DemoTaskDescription::new()),
                ModifiedTaskDependencies::new().on_self(),
            );
        }

        TaskOutput::with_output(DemoTaskOutput {
            was_even: self.delay_before + self.delay_after % 2 == 0,
        })
        .modifications(modifications)
    }
}

#[derive(Clone)]
pub struct DemoTaskOutput {
    was_even: bool,
}

impl Output for DemoTaskOutput {
    fn as_any(&self) -> Box<dyn Any> {
        Box::new(self.clone()) // TODO: Avoid clone?
    }
}
