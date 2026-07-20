use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{
    Modifications, ModifiedTaskDependencies, OutputsOfDependencies, Task, TaskDescription,
    TaskOutput,
};
use crate::tasks::demo_task::DemoTaskDescription;
use crate::tool_runner::ToolRunner;

pub struct SetupTaskDescription {}

impl SetupTaskDescription {
    pub fn new() -> Self {
        Self {}
    }
}
impl TaskDescription for SetupTaskDescription {
    fn name(&self) -> String {
        "SetupTask".to_string()
    }

    fn create(&self) -> Box<dyn Task> {
        Box::new(SetupTask {})
    }
}

struct SetupTask {}

#[async_trait::async_trait]
impl Task for SetupTask {
    async fn run(
        &mut self,
        model: PrismModel,
        properties: PropertyCollection,
        inputs: OutputsOfDependencies,
        tool_runner: ToolRunner,
    ) -> TaskOutput {
        let _ = (model, properties, inputs, tool_runner);
        let mut modifications = Modifications::new();
        modifications.create_task(
            Box::new(DemoTaskDescription::new()),
            ModifiedTaskDependencies::new().on_self(),
        );

        // let checking_task = modifications.create_task(
        //     Box::new(ModelCheckingTaskDescription::new()),
        //     ModifiedTaskDependencies::new().on_self(),
        // );
        // modifications.create_task(
        //     Box::new(SetupRepairEnginesTaskDescription::new()),
        //     ModifiedTaskDependencies::new().on(checking_task),
        // );
        TaskOutput {
            output: Box::new(()),
            modifications,
        }
    }
}
