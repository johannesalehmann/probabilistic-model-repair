use crate::prism_runner::check_properties;
use crate::repair_graph::{CheckingResult, PrismModel, PropertyCollection};
use crate::task_graph::{Modifications, OutputsOfDependencies, Task, TaskDescription, TaskOutput};
use crate::tool_runner::ToolRunner;

pub struct ModelCheckingTaskDescription {}

impl ModelCheckingTaskDescription {
    pub fn new() -> Self {
        Self {}
    }
}

impl TaskDescription for ModelCheckingTaskDescription {
    fn name(&self) -> String {
        "ModelCheckingTask".to_string()
    }

    fn create(&self) -> Box<dyn Task> {
        Box::new(ModelCheckingTask {})
    }
}

struct ModelCheckingTask {}

#[async_trait::async_trait]
impl Task for ModelCheckingTask {
    async fn run(
        &mut self,
        model: PrismModel,
        properties: PropertyCollection,
        inputs: OutputsOfDependencies,
        mut tool_runner: ToolRunner,
    ) -> TaskOutput {
        let results = check_properties(&model, &properties, &mut tool_runner).await;
        let mut all_bools_satisfied = true;
        for result in &results.results {
            match result {
                CheckingResult::Bool(false) => {
                    all_bools_satisfied = false;
                }
                _ => (),
            }
        }
        let mut modifications = Modifications::new();
        if all_bools_satisfied {
            modifications.announce_completion();
        }
        TaskOutput::new().modifications(modifications)
    }
}
