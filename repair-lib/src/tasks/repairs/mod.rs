use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{
    Modifications, ModifiedTaskDependencies, OutputsOfDependencies, Task, TaskDescription,
    TaskOutput,
};
use crate::tool_runner::ToolRunner;
use std::any::Any;
use std::path::Path;

mod syntactic_replacement;
// mod synthesis;

pub struct SetupRepairEnginesTaskDescription {}
impl SetupRepairEnginesTaskDescription {
    pub fn new() -> Self {
        Self {}
    }
}

impl TaskDescription for SetupRepairEnginesTaskDescription {
    fn name(&self) -> String {
        "SetupRepairEnginesTask".to_string()
    }

    fn create(&self) -> Box<dyn Task> {
        Box::new(SetupRepairEnginesTask {})
    }
}

pub struct SetupRepairEnginesTask {}

#[async_trait::async_trait]
impl Task for SetupRepairEnginesTask {
    async fn run(
        &mut self,
        model: PrismModel,
        properties: PropertyCollection,
        inputs: OutputsOfDependencies,
        tool_runner: ToolRunner,
    ) -> TaskOutput {
        let _ = (model, properties, inputs, tool_runner);
        let mut modifications = Modifications::new();
        // modifications.create_task(
        //     Box::new(synthesis::SetupTask::new()),
        //     ModifiedTaskDependencies::new().on_self(),
        // );
        TaskOutput {
            output: Box::new(()),
            modifications,
        }
    }
}
