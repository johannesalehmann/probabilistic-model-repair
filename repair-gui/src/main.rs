pub mod controls;
mod input;
mod ui;

use crate::input::Paths;
use iced::advanced::image::Handle;
use iced::daemon::ViewFn;
use iced::{Element, Task};
use repair_lib::repair_problem::{ProgressKind, RepairProblem};
use tabbed_workspace::TabbedWorkspace;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    iced::run(Window::update, Window::view).expect("Failed to run UI");
}

struct Window {
    workspace: TabbedWorkspace<TabWindow>,
    shared_state: SharedState,
    update_watcher: Option<mpsc::Receiver<ProgressKind>>, // Only stores the receiver if it is not stored in a separate thread.
}

struct SharedState {
    repair_problem: RepairProblem,
    repair_graph_layout: ui::repair_graph::RepairGraphLayout,
}

impl SharedState {
    pub fn new(repair_problem: RepairProblem) -> Self {
        let mut repair_graph_layout = ui::repair_graph::RepairGraphLayout::new();
        repair_graph_layout.update_for_graph(&repair_problem.graph.lock().unwrap());
        Self {
            repair_problem,
            repair_graph_layout,
        }
    }
}

impl Default for Window {
    fn default() -> Self {
        let paths = Paths::search_directory("models/synthesis_input_variable/");
        let description = match input::get_description(&paths) {
            Ok(description) => description,
            Err(err) => {
                err.print_error();
                panic!();
            }
        };
        let (task, update_watcher) = description.start(Some(4));
        let shared_state = SharedState::new(task);

        let mut workspace = TabbedWorkspace::new();
        workspace.open_window(TabWindow::RepairGraph(
            ui::repair_graph::RepairGraphUITab::new(),
        ));
        workspace.open_window(TabWindow::TaskOverview(
            ui::task_overview::TaskViewTab::new(),
        ));

        Self {
            workspace,
            shared_state,
            update_watcher: Some(update_watcher),
        }
    }
}

impl Window {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = match message {
            Message::Workspace(msg) => self
                .workspace
                .update(msg, &mut self.shared_state)
                .map(Message::Workspace),
            Message::UpdateReceived { update_receiver } => {
                self.update_watcher = Some(update_receiver);
                self.shared_state
                    .repair_graph_layout
                    .update_for_graph(&self.shared_state.repair_problem.graph.lock().unwrap());
                Task::none()
            }
        };
        if let Some(receiver) = self.update_watcher.take() {
            let watcher = UpdateWatcher {
                update_receiver: receiver,
            };
            let monitor_task = Task::future(watcher.watch());
            Task::<Message>::batch([monitor_task, task])
        } else {
            task
        }
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        self.workspace.view(&self.shared_state, Message::Workspace)
    }
}

enum Message {
    Workspace(tabbed_workspace::Message<TabAction>),
    UpdateReceived {
        update_receiver: mpsc::Receiver<ProgressKind>,
    },
}

enum TabWindow {
    RepairGraph(ui::repair_graph::RepairGraphUITab),
    TaskOverview(ui::task_overview::TaskViewTab),
}

#[derive(Clone)]
enum TabAction {
    RepairGraphMessage(ui::repair_graph::RepairGraphMessage),
    TaskOverviewMessage(ui::task_overview::TaskViewMessage),
}

impl tabbed_workspace::Window for TabWindow {
    type SharedState = SharedState;
    type TabAction = TabAction;

    fn title(&self, shared_state: &SharedState) -> String {
        match self {
            TabWindow::RepairGraph(graph) => "Repair graph".to_string(),
            TabWindow::TaskOverview(graph) => "Task overview".to_string(),
        }
    }

    fn icon(&self, shared_state: &SharedState) -> Option<Handle> {
        match self {
            // create an image as follows:
            // Some(Handle::from_path("repair-gui/resources/icons/prism_logo.png")),
            TabWindow::RepairGraph(graph) => None,
            TabWindow::TaskOverview(overview) => None,
        }
    }

    fn update(&mut self, action: TabAction, shared_state: &mut SharedState) -> Task<TabAction> {
        match action {
            TabAction::RepairGraphMessage(message) => {
                if let TabWindow::RepairGraph(graph) = self {
                    graph.update(shared_state, message);
                    Task::none()
                } else {
                    panic!("Tried to perform repair graph action on incorrect tab type")
                }
            }
            TabAction::TaskOverviewMessage(message) => {
                if let TabWindow::TaskOverview(task_overview) = self {
                    task_overview.update(shared_state, message);
                    Task::none()
                } else {
                    panic!("Tried to perform task overview action on incorrect tab type")
                }
            }
        }
    }

    fn view<'a>(&'a self, shared_state: &SharedState) -> Element<'a, TabAction> {
        match self {
            TabWindow::RepairGraph(graph) => {
                graph.view(shared_state).map(TabAction::RepairGraphMessage)
            }
            TabWindow::TaskOverview(task_overview) => task_overview
                .view(shared_state)
                .map(TabAction::TaskOverviewMessage),
        }
    }
}

struct UpdateWatcher {
    update_receiver: mpsc::Receiver<ProgressKind>,
}

impl UpdateWatcher {
    pub async fn watch(mut self) -> Message {
        self.update_receiver.recv().await;
        Message::UpdateReceived {
            update_receiver: self.update_receiver,
        }
    }
}
