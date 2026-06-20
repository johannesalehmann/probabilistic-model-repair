use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{DependencyOutputs, Modifications, Task};
use std::any::Any;

mod syntactic_replacement;
mod synthesis;

pub struct SetupRepairEnginesTask {}
impl SetupRepairEnginesTask {
    pub fn new() -> Self {
        Self {}
    }
}

impl Task for SetupRepairEnginesTask {
    fn description(&self) -> String {
        "SetupRepairEnginesTask".to_string()
    }

    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: DependencyOutputs,
        modifications: &mut Modifications,
    ) -> Box<dyn Any> {
        modifications.create_task(Box::new(synthesis::SetupTask::new()), vec![own_index]);

        Box::new(())
    }
}
