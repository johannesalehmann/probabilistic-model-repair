mod parameters;
pub use parameters::*;

use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::tool_runner::ToolRunner;
use std::any::Any;
use std::path::Path;

pub struct TaskGraph {
    pub tasks: Vec<TaskGraphNode>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add_task<T: TaskDescription + 'static>(&mut self, task: T, dependencies: Vec<usize>) {
        self.tasks.push(TaskGraphNode {
            description: Box::new(task),
            dependencies,
            status: TaskStatus::Ready,
        })
    }

    pub fn is_task_ready(&self, task_index: usize) -> bool {
        if self.tasks[task_index].ready_to_run() {
            for dependency in &self.tasks[task_index].dependencies {
                if !self.tasks[*dependency].is_done() {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    pub fn dependency_outputs(&self, task_index: usize) -> OutputsOfDependencies {
        let mut dependency_outputs = OutputsOfDependencies::new();
        for (dependency_index, &dependency) in
            self.tasks[task_index].dependencies.iter().enumerate()
        {
            match &self.tasks[dependency].status {
                TaskStatus::Ready => {
                    panic!("Cannot run a task that depends on a task that has not yet been run.")
                }
                TaskStatus::Running { .. } => {
                    panic!("Cannot run a task that depends on a task that is currently running.")
                }
                TaskStatus::Done { output, .. } => dependency_outputs
                    .outputs
                    .push((dependency_index, output.clone())),
            }
        }
        dependency_outputs
    }
}

pub struct TaskGraphNode {
    pub description: Box<dyn TaskDescription>,
    pub dependencies: Vec<usize>,
    pub status: TaskStatus,
}

impl TaskGraphNode {
    pub fn ready_to_run(&self) -> bool {
        if let TaskStatus::Ready = self.status {
            true
        } else {
            false
        }
    }

    pub fn is_done(&self) -> bool {
        self.task_output().is_some()
    }

    pub fn task_output(&self) -> Option<&Box<dyn Output>> {
        if let TaskStatus::Done { output, .. } = &self.status {
            Some(output)
        } else {
            None
        }
    }
}

pub trait TaskDescription: Send {
    fn name(&self) -> String;

    fn parameter_descriptions(&self) -> Vec<ParameterDescription> {
        Vec::new()
    }

    fn parameter_value(&self, index: usize) -> ParameterValue {
        panic!("Task has no parameter with index {index}")
    }
    fn set_parameter_value(&mut self, index: usize, value: ParameterValue) {
        panic!("Task has no parameter with index {index}")
    }
    fn parameter_summary(&self) -> String {
        "no parameters".to_string()
    }

    fn create(&self) -> Box<dyn Task>;
}

#[async_trait::async_trait]
pub trait Task: Send {
    async fn run(
        &mut self,
        model: PrismModel,
        properties: PropertyCollection,
        inputs: OutputsOfDependencies,
        tool_runner: ToolRunner,
    ) -> TaskOutput;
}

pub enum TaskStatus {
    Ready,
    Running {
        handle: tokio::task::JoinHandle<()>,
        start_time: std::time::Instant,
    },
    Done {
        output: Box<dyn Output>,
        elapsed: std::time::Duration,
    },
}

pub trait Output: CloneOutput + Send {
    fn as_any(&self) -> Box<dyn Any>;
}
pub trait CloneOutput {
    fn clone_output<'a>(&self) -> Box<dyn Output>;
}

impl<T> CloneOutput for T
where
    T: Output + Clone + 'static,
{
    fn clone_output<'a>(&self) -> Box<dyn Output> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Output> {
    fn clone(&self) -> Self {
        self.clone_output()
    }
}

impl Output for () {
    fn as_any(&self) -> Box<dyn Any> {
        Box::new(())
    }
}

pub struct TaskOutput {
    pub output: Box<dyn Output>,
    pub modifications: Modifications,
}

impl TaskOutput {
    pub fn new() -> TaskOutput {
        Self {
            output: Box::new(()),
            modifications: Modifications::new(),
        }
    }

    pub fn with_output<T: Output + 'static>(output: T) -> Self {
        Self {
            output: Box::new(output),
            modifications: Modifications::new(),
        }
    }

    pub fn modifications(mut self, modifications: Modifications) -> Self {
        self.modifications = modifications;
        self
    }
}

pub enum ExternalChange {
    CreateRepair {
        model: PrismModel,
        properties: PropertyCollection,
    },
    AnnounceCompletion,
}

pub enum OpsGraphChange {
    AddNode { task: Box<dyn TaskDescription> },
    ExternalChange(ExternalChange),
}

pub struct OutputsOfDependencies {
    outputs: Vec<(usize, Box<dyn Output>)>,
}

impl OutputsOfDependencies {
    pub fn new() -> Self {
        Self {
            outputs: Vec::new(),
        }
    }

    pub fn get<T: 'static>(&self) -> Option<(usize, &T)> {
        for (dependency_index, output) in &self.outputs {
            if let Ok(res) = output.as_any().downcast() {
                return Some((*dependency_index, *res));
            }
        }
        None
    }
}

pub struct ModifiedTaskDependencies {
    dependencies: Vec<ModifiedTaskDependency>,
}

impl ModifiedTaskDependencies {
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }

    pub fn add(&mut self, dependency: ModifiedTaskDependency) {
        self.dependencies.push(dependency)
    }

    pub fn on_self(mut self) -> Self {
        self.dependencies.push(ModifiedTaskDependency::Parent);
        self
    }

    pub fn on_parent_dependency(mut self, parent_dependency_index: usize) -> Self {
        self.dependencies
            .push(ModifiedTaskDependency::ParentDependency {
                parent_dependency_index,
            });
        self
    }

    pub fn on(mut self, dependency: ModifiedTaskDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    pub fn as_indices(
        &self,
        parent_dependencies: &[usize],
        parent_index: usize,
        first_new_task_index: usize,
    ) -> Vec<usize> {
        let mut res = Vec::new();

        for dependency in &self.dependencies {
            match dependency {
                ModifiedTaskDependency::ParentDependency {
                    parent_dependency_index,
                } => res.push(parent_dependencies[*parent_dependency_index]),
                ModifiedTaskDependency::Parent => {
                    res.push(parent_index);
                }
                ModifiedTaskDependency::NewTask { new_task_index } => {
                    res.push(first_new_task_index + new_task_index)
                }
            }
        }

        res
    }
}

pub enum ModifiedTaskDependency {
    ParentDependency { parent_dependency_index: usize },
    Parent,
    NewTask { new_task_index: usize },
}

pub struct Modifications {
    pub new_tasks: Vec<(Box<dyn TaskDescription>, ModifiedTaskDependencies)>,
    pub external_changes: Vec<ExternalChange>,
}

impl Modifications {
    pub fn new() -> Self {
        Self {
            new_tasks: Vec::new(),
            external_changes: Vec::new(),
        }
    }

    pub fn create_task(
        &mut self,
        task: Box<dyn TaskDescription>,
        dependencies: ModifiedTaskDependencies,
    ) -> ModifiedTaskDependency {
        let res = ModifiedTaskDependency::NewTask {
            new_task_index: self.new_tasks.len(),
        };
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
