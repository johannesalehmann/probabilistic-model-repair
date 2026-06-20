use crate::prism_runner::check_properties;
use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{DependencyOutputs, Modifications, Task};
use std::any::Any;

pub struct ModelCheckingTask {}

impl ModelCheckingTask {
    pub fn new() -> Self {
        Self {}
    }
}

impl Task for ModelCheckingTask {
    fn description(&self) -> String {
        "ModelCheckingTask".to_string()
    }

    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: DependencyOutputs,
        modifications: &mut Modifications,
    ) -> Box<dyn Any> {
        let results = check_properties(model, properties);
        Box::new(results)
    }
}
