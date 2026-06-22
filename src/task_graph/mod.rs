use crate::repair_graph::{PrismModel, PropertyCollection};
use std::any::Any;
use std::path::Path;

pub struct TaskGraph {
    tasks: Vec<TaskGraphNode>,
    outputs: Vec<Option<Box<dyn Any>>>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: vec![TaskGraphNode {
                task: Box::new(crate::tasks::SetupTask::new()),
                depends_on: vec![],
            }],
            outputs: vec![None],
        }
    }

    pub fn get_executable(&self) -> Option<usize> {
        for (index, (task, output)) in self.tasks.iter().zip(self.outputs.iter()).enumerate() {
            let executable =
                output.is_none() && task.depends_on.iter().all(|i| self.outputs[*i].is_some());
            if executable {
                return Some(index);
            }
        }
        None
    }

    pub fn execute(
        &mut self,
        index: usize,
        model: &PrismModel,
        properties: &PropertyCollection,
        temp_directory: &Path,
    ) -> Vec<ExternalChange> {
        println!(
            "Executing task {} ({})",
            index,
            self.tasks[index].task.description()
        );
        if self.outputs[index].is_some() {
            panic!("Cannot execute the same node twice");
        }
        let mut dependency_outputs = DependencyOutputs::new();
        for &dependency in &self.tasks[index].depends_on {
            match self.outputs[dependency].as_ref() {
                None => panic!("Cannot execute task {index} before task {dependency} is executed."),
                Some(output) => dependency_outputs.outputs.push((dependency, output)),
            }
        }
        let mut modifications = Modifications::new(self.tasks.len());
        let output = self.tasks[index].task.run(
            model,
            properties,
            index,
            dependency_outputs,
            &mut modifications,
            temp_directory,
        );
        self.outputs[index] = Some(output);

        for (new_task, depends_on) in modifications.new_tasks {
            println!("Creating new task ({})", new_task.description());
            self.tasks.push(TaskGraphNode {
                task: new_task,
                depends_on,
            });
            self.outputs.push(None);
        }

        modifications.external_changes
    }
}

pub struct TaskGraphNode {
    task: Box<dyn Task>,
    depends_on: Vec<usize>,
}

pub trait Task {
    fn description(&self) -> String;
    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: DependencyOutputs,
        modifications: &mut Modifications,
        temp_directory: &Path,
    ) -> Box<dyn Any>;
}

pub enum ExternalChange {
    CreateRepair {
        model: PrismModel,
        properties: PropertyCollection,
    },
    AnnounceCompletion,
}

pub enum OpsGraphChange {
    AddNode { task: Box<dyn Task> },
    ExternalChange(ExternalChange),
}

pub struct DependencyOutputs<'a> {
    outputs: Vec<(usize, &'a Box<dyn Any>)>,
}

impl<'a> DependencyOutputs<'a> {
    pub fn new() -> Self {
        Self {
            outputs: Vec::new(),
        }
    }

    pub fn get<T: 'static>(&self) -> Option<(usize, &T)> {
        for (dependency_index, output) in &self.outputs {
            if let Some(res) = output.downcast_ref() {
                return Some((*dependency_index, res));
            }
        }
        None
    }
}

pub struct Modifications {
    task_index_offset: usize,
    new_tasks: Vec<(Box<dyn Task>, Vec<usize>)>,
    external_changes: Vec<ExternalChange>,
}

impl Modifications {
    pub fn new(task_index_offset: usize) -> Self {
        Self {
            task_index_offset,
            new_tasks: Vec::new(),
            external_changes: Vec::new(),
        }
    }

    pub fn create_task(&mut self, task: Box<dyn Task>, dependencies: Vec<usize>) -> usize {
        let res = self.new_tasks.len() + self.task_index_offset;
        self.new_tasks.push((task, dependencies));
        res
    }

    pub fn create_repair_graph_node(&mut self, model: PrismModel, properties: PropertyCollection) {
        self.external_changes
            .push(ExternalChange::CreateRepair { model, properties })
    }

    pub fn announce_completion(&mut self) {
        self.external_changes
            .push(ExternalChange::AnnounceCompletion)
    }
}
