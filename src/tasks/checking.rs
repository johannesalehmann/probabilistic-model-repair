use crate::prism_runner::check_properties;
use crate::repair_graph::{CheckingResult, PrismModel, PropertyCollection};
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
        let mut all_bools_satisfied = true;
        for result in &results.results {
            match result {
                CheckingResult::Bool(false) => {
                    all_bools_satisfied = false;
                }
                _ => (),
            }
        }
        if all_bools_satisfied {
            modifications.announce_completion();
        }
        Box::new(results)
    }
}
