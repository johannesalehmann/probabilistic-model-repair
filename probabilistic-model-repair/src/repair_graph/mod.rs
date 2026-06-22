use crate::task_graph::{ExternalChange, TaskGraph};
use std::path::Path;

pub type PrismModel = prism_model::Model;
type PrismProperty = probabilistic_properties::Query<
    prism_model::Expression,
    prism_model::Expression,
    prism_model::Expression,
>;

pub struct RepairGraph {
    pub nodes: Vec<RepairGraphNode>,
}

impl RepairGraph {
    pub fn with_initial_node(prism_model: PrismModel, properties: PropertyCollection) -> Self {
        Self {
            nodes: vec![RepairGraphNode::new(prism_model, properties)],
        }
    }
}

pub struct RepairGraphNode {
    pub model: PrismModel,
    // TODO: I'm not sure it makes sense to store properties in here. On the other hand, the
    //  property explication engine (which is to do) might modify properties of the model in
    //  repair nodes.
    pub properties: PropertyCollection,
    pub tasks: TaskGraph,
}

impl RepairGraphNode {
    pub fn new(prism_model: PrismModel, properties: PropertyCollection) -> Self {
        Self {
            model: prism_model,
            properties,
            tasks: TaskGraph::new(),
        }
    }

    pub fn execute_task(&mut self, index: usize, temp_directory: &Path) -> Vec<ExternalChange> {
        self.tasks
            .execute(index, &self.model, &self.properties, temp_directory)
    }
}

#[derive(Clone)]
pub struct PropertyCollection {
    pub properties: Vec<PrismProperty>,
}

impl PropertyCollection {
    pub fn new(properties: Vec<PrismProperty>) -> Self {
        Self { properties }
    }
}

pub struct CheckingResults {
    pub results: Vec<CheckingResult>,
}

impl CheckingResults {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
}

pub enum CheckingResult {
    Bool(bool),
    Float(f64),
}
