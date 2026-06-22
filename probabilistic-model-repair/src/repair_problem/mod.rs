use crate::repair_graph::{PrismModel, PropertyCollection, RepairGraph, RepairGraphNode};
use crate::task_graph::ExternalChange;
use std::env::temp_dir;
use std::path::{Path, PathBuf};

pub struct RepairProblemDescription {
    model: PrismModel,
    properties: PropertyCollection,
    temp_directory: PathBuf,
}

impl RepairProblemDescription {
    pub fn new(model: PrismModel, properties: PropertyCollection, temp_directory: PathBuf) -> Self {
        Self {
            model,
            properties,
            temp_directory,
        }
    }

    pub fn build(mut self) -> RepairProblem {
        crate::preprocessing::preprocess_model(&mut self.model);

        RepairProblem {
            graph: RepairGraph::with_initial_node(self.model, self.properties),
            temp_directory: self.temp_directory,
        }
    }
}

pub struct RepairProblem {
    graph: RepairGraph,
    temp_directory: PathBuf,
}

impl RepairProblem {
    pub fn step(&mut self) -> StepResult {
        let mut more_to_do = false;
        for i in self.graph.nodes.len() - 1..self.graph.nodes.len() {
            if let Some(task) = self.graph.nodes[i].tasks.get_executable() {
                let changes = self.graph.nodes[i].execute_task(task, &self.temp_directory);
                for change in changes {
                    match change {
                        ExternalChange::CreateRepair { model, properties } => {
                            self.graph
                                .nodes
                                .push(RepairGraphNode::new(model, properties));
                        }
                        ExternalChange::AnnounceCompletion => {
                            return StepResult::Done {
                                model: self.graph.nodes[i].model.clone(),
                                properties: self.graph.nodes[i].properties.clone(),
                            };
                        }
                    }
                }
                more_to_do = true;
            }
        }

        match more_to_do {
            true => StepResult::MoreToDo,
            false => StepResult::NoMoreTasks,
        }
    }
}

pub enum StepResult {
    Done {
        model: PrismModel,
        properties: PropertyCollection,
    },
    MoreToDo,
    NoMoreTasks,
}
