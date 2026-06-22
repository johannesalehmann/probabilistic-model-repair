use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{DependencyOutputs, Modifications, Task};
use crate::tasks::ModelCheckingTask;
use crate::tasks::repairs::SetupRepairEnginesTask;
use std::any::Any;
use std::path::Path;

pub struct SetupTask {}

impl SetupTask {
    pub fn new() -> Self {
        Self {}
    }
}

impl Task for SetupTask {
    fn description(&self) -> String {
        "SetupTask".to_string()
    }
    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: DependencyOutputs,
        modifications: &mut Modifications,
        temp_directory: &Path,
    ) -> Box<dyn Any> {
        let _ = (model, properties, dependency_outputs);
        let checking_task =
            modifications.create_task(Box::new(ModelCheckingTask::new()), vec![own_index]);
        modifications.create_task(Box::new(SetupRepairEnginesTask::new()), vec![checking_task]);
        Box::new(())
    }
}
