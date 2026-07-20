use crate::task_graph::{ExternalChange, TaskGraph, TaskGraphNode, TaskOutput, TaskStatus};
use crate::tasks::SetupTaskDescription;
use crate::tool_runner::{LogUpdate, MainToolRunner};
use tokio::sync::mpsc;

pub type PrismModel = prism_model::Model;
type PrismProperty = probabilistic_properties::Query<
    prism_model::Expression,
    prism_model::Expression,
    prism_model::Expression,
>;

pub struct RepairGraph {
    pub nodes: Vec<RepairGraphNode>,
    pub sender: mpsc::Sender<WrappedTaskOutput>,
    pub tool_runner: MainToolRunner,
}

impl RepairGraph {
    pub fn with_initial_node(
        prism_model: PrismModel,
        properties: PropertyCollection,
    ) -> (
        Self,
        mpsc::Receiver<WrappedTaskOutput>,
        mpsc::Receiver<LogUpdate>,
    ) {
        let (sender, receiver) = mpsc::channel(32);

        let (tool_runner, tool_receiver) = MainToolRunner::new();

        (
            Self {
                nodes: vec![RepairGraphNode::new(prism_model, properties, None)],
                sender,
                tool_runner,
            },
            receiver,
            tool_receiver,
        )
    }

    pub fn run_task(&mut self, model_index: usize, task_index: usize) {
        let model_node = &self.nodes[model_index];
        let task_node = &model_node.tasks.tasks[task_index];

        let dependency_outputs = model_node.tasks.dependency_outputs(task_index);

        let model = model_node.model.clone();
        let properties = model_node.properties.clone(); // TODO: avoid these clones

        let sender = self.sender.clone();

        let mut task = task_node.description.create();
        let tool_runner = self.tool_runner.get_tool_runner(model_index, task_index);

        let run = async move || {
            let result = task
                .run(model, properties, dependency_outputs, tool_runner)
                .await;
            sender
                .send(WrappedTaskOutput {
                    output: result,
                    model_index,
                    task_index,
                })
                .await
                .expect("Task could not send its results");
        };
        let handle = tokio::task::spawn(run());

        self.nodes[model_index].tasks.tasks[task_index].status = TaskStatus::Running {
            handle,
            start_time: std::time::Instant::now(),
        };
    }

    pub fn process_output(&mut self, result: WrappedTaskOutput) {
        let task = &mut self.nodes[result.model_index].tasks.tasks[result.task_index];
        let elapsed = if let TaskStatus::Running { start_time, .. } = task.status {
            start_time.elapsed()
        } else {
            panic!("Task completed that previously was not running");
        };
        task.status = TaskStatus::Done {
            output: result.output.output,
            elapsed,
        };

        let parent_dependencies = task.dependencies.clone(); // TODO: Avoid this clone
        let first_new_task_index = self.nodes[result.model_index].tasks.tasks.len();
        for (new_task, new_dependencies) in result.output.modifications.new_tasks {
            let dependencies = new_dependencies.as_indices(
                &parent_dependencies,
                result.task_index,
                first_new_task_index,
            );
            self.nodes[result.model_index]
                .tasks
                .tasks
                .push(TaskGraphNode {
                    description: new_task,
                    dependencies,
                    status: TaskStatus::Ready,
                })
        }

        for external_change in result.output.modifications.external_changes {
            match external_change {
                ExternalChange::CreateRepair { model, properties } => {
                    self.nodes.push(RepairGraphNode::new(
                        model,
                        properties,
                        Some(RepairGraphParent::new(
                            result.model_index,
                            result.task_index,
                        )),
                    ))
                }
                ExternalChange::AnnounceCompletion => {
                    // TODO
                }
            }
        }
    }
}

pub enum WaitResult {
    Completed,
    NoMoreTasks,
    MoreToDo,
}

pub struct WrappedTaskOutput {
    output: TaskOutput,
    model_index: usize,
    task_index: usize,
}

pub struct RepairGraphParent {
    pub model_index: usize,
    pub task_index: usize,
}

impl RepairGraphParent {
    pub fn new(model_index: usize, task_index: usize) -> Self {
        Self {
            model_index,
            task_index,
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
    pub parent: Option<RepairGraphParent>,
}

impl RepairGraphNode {
    pub fn new(
        prism_model: PrismModel,
        properties: PropertyCollection,
        parent: Option<RepairGraphParent>,
    ) -> Self {
        let mut tasks = TaskGraph::new();
        tasks.add_task(SetupTaskDescription {}, Vec::new());
        Self {
            model: prism_model,
            properties,
            tasks,
            parent,
        }
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
