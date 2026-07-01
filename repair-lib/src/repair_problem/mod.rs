use crate::repair_graph::{
    PrismModel, PropertyCollection, RepairGraph, RepairGraphNode, RepairGraphParent,
    WrappedTaskOutput,
};
use crate::tool_runner::ToolUpdate;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

pub struct RepairProblemDescription {
    model: PrismModel,
    properties: PropertyCollection,
}

impl RepairProblemDescription {
    pub fn new(model: PrismModel, properties: PropertyCollection) -> Self {
        Self { model, properties }
    }

    pub fn start(mut self) -> (RepairProblem, mpsc::Receiver<ProgressKind>) {
        crate::preprocessing::preprocess_model(&mut self.model);
        let (update_sender, update_receiver) = mpsc::channel(64);

        let (graph, task_updates, tool_updates) =
            RepairGraph::with_initial_node(self.model, self.properties);
        let graph = Arc::new(Mutex::new(graph));

        let task_watcher =
            TaskFinishedWatcher::new(update_sender.clone(), graph.clone(), task_updates);
        let tool_watcher = ToolCallWatcher::new(update_sender, graph.clone(), tool_updates);

        tokio::spawn(task_watcher.run());
        tokio::spawn(tool_watcher.run());

        (RepairProblem { graph }, update_receiver)
    }
}

pub struct RepairProblem {
    pub graph: Arc<Mutex<RepairGraph>>,
}

pub enum ProgressKind {
    TaskFinished,
    ToolCall,
}

pub struct TaskFinishedWatcher {
    update_sender: mpsc::Sender<ProgressKind>,
    graph: Arc<Mutex<RepairGraph>>,
    task_update_receiver: mpsc::Receiver<WrappedTaskOutput>,
}

impl TaskFinishedWatcher {
    pub fn new(
        update_sender: mpsc::Sender<ProgressKind>,
        graph: Arc<Mutex<RepairGraph>>,
        task_update_receiver: mpsc::Receiver<WrappedTaskOutput>,
    ) -> Self {
        Self {
            update_sender,
            graph,
            task_update_receiver,
        }
    }

    pub async fn run(mut self) {
        while let Some(update) = self.task_update_receiver.recv().await {
            {
                let mut graph = self.graph.lock().unwrap();
                graph.process_output(update);
            }
            self.update_sender
                .send(ProgressKind::TaskFinished)
                .await
                .unwrap();
        }
    }
}

pub struct ToolCallWatcher {
    update_sender: mpsc::Sender<ProgressKind>,
    graph: Arc<Mutex<RepairGraph>>,
    tool_call_update_receiver: mpsc::Receiver<ToolUpdate>,
}

impl ToolCallWatcher {
    pub fn new(
        update_sender: mpsc::Sender<ProgressKind>,
        graph: Arc<Mutex<RepairGraph>>,
        tool_call_update_receiver: mpsc::Receiver<ToolUpdate>,
    ) -> Self {
        Self {
            update_sender,
            graph,
            tool_call_update_receiver,
        }
    }

    pub async fn run(mut self) {
        while let Some(update) = self.tool_call_update_receiver.recv().await {
            {
                let mut graph = self.graph.lock().unwrap();
                graph.tool_runner.process_update(update);
            }
            self.update_sender
                .send(ProgressKind::ToolCall)
                .await
                .unwrap();
        }
    }
}
