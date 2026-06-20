use crate::repair_graph::{PrismModel, PropertyCollection, RepairGraph, RepairGraphNode};
use crate::task_graph::ExternalChange;

pub struct RepairProblemDescription {
    model: PrismModel,
    properties: PropertyCollection,
}

impl RepairProblemDescription {
    pub fn new(model: PrismModel, properties: PropertyCollection) -> Self {
        Self { model, properties }
    }

    pub fn build(mut self) -> RepairProblem {
        crate::preprocessing::preprocess_model(&mut self.model);

        RepairProblem {
            graph: RepairGraph::with_initial_node(self.model, self.properties),
        }
    }
}

pub struct RepairProblem {
    graph: RepairGraph,
}

impl RepairProblem {
    pub fn step(&mut self) -> StepResult {
        if self.graph.nodes.len() > 1 {
            panic!(
                "The scheduler does not yet know how to decide between multiple nodes in the graph"
            );
        }
        if let Some(task) = self.graph.nodes[0].tasks.get_executable() {
            let changes = self.graph.nodes[0].execute_task(task);
            for change in changes {
                match change {
                    ExternalChange::CreateRepair { model, properties } => {
                        self.graph
                            .nodes
                            .push(RepairGraphNode::new(model, properties));
                    }
                    ExternalChange::AnnounceCompletion => return StepResult::Done,
                }
            }
            StepResult::MoreToDo
        } else {
            StepResult::NoMoreTasks
        }
    }
}

pub enum StepResult {
    Done,
    MoreToDo,
    NoMoreTasks,
}
