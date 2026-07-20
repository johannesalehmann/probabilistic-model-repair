use crate::repair_graph::{PrismModel, PropertyCollection, RepairGraph, WrappedTaskOutput};
use crate::tool_runner::LogUpdate;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct RepairProblemDescription {
    model: PrismModel,
    properties: PropertyCollection,
}

impl RepairProblemDescription {
    pub fn new(model: PrismModel, properties: PropertyCollection) -> Self {
        Self { model, properties }
    }

    pub fn start(
        mut self,
        ticks_per_second: Option<usize>,
    ) -> (RepairProblem, mpsc::Receiver<ProgressKind>) {
        crate::preprocessing::preprocess_model(&mut self.model);
        let (update_sender, update_receiver) = mpsc::channel(64);

        let (graph, task_updates, tool_updates) =
            RepairGraph::with_initial_node(self.model, self.properties);
        let graph = Arc::new(Mutex::new(graph));

        if let Some(ticks_per_second) = ticks_per_second
            && ticks_per_second > 0
        {
            let ticker = Ticker::new(ticks_per_second, update_sender.clone());
            tokio::spawn(ticker.run());
        }

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
    Tick,
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
    tool_call_update_receiver: mpsc::Receiver<LogUpdate>,
}

impl ToolCallWatcher {
    pub fn new(
        update_sender: mpsc::Sender<ProgressKind>,
        graph: Arc<Mutex<RepairGraph>>,
        tool_call_update_receiver: mpsc::Receiver<LogUpdate>,
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

struct Ticker {
    period: Duration,
    update_sender: mpsc::Sender<ProgressKind>,
}

impl Ticker {
    pub fn new(ticks_per_second: usize, update_sender: mpsc::Sender<ProgressKind>) -> Self {
        Self {
            period: Duration::from_secs_f32(1.0 / ticks_per_second as f32),
            update_sender,
        }
    }

    pub async fn run(self) {
        while self.update_sender.send(ProgressKind::Tick).await.is_ok() {
            tokio::time::sleep(self.period).await;
        }
    }
}
