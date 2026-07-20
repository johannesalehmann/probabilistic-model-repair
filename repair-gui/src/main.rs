mod input;
mod ui;

use crate::input::Paths;
use crate::ui::repair_graph::{RepairGraphMessage, RepairGraphUITab};
use iced::advanced::image::Handle;
use iced::advanced::widget::Operation;
use iced::futures::{AsyncBufReadExt, StreamExt};
use iced::widget::text::Highlighter;
use iced::widget::{TextEditor, text, text_editor};
use iced::{Element, Task};
use repair_lib::repair_problem::{ProgressKind, RepairProblem};
use std::fmt::Debug;
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
        workspace.open_window(TabWindow::code_window());
        workspace.open_window(TabWindow::MdpExplorer);
        workspace.open_window(TabWindow::RepairGraph(RepairGraphUITab::new()));

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
    SourceCode { content: String },
    CodeWindow { content: text_editor::Content },
    RepairGraph(RepairGraphUITab),
    MdpExplorer,
}

impl TabWindow {
    pub fn code_window() -> Self {
        Self::CodeWindow {
            content: text_editor::Content::new(),
        }
    }
}

#[derive(Clone)]
enum TabAction {
    EditCode(text_editor::Action),
    RepairGraphMessage(RepairGraphMessage),
}

impl tabbed_workspace::Window for TabWindow {
    type SharedState = SharedState;
    type TabAction = TabAction;

    fn title(&self, shared_state: &SharedState) -> String {
        match self {
            TabWindow::CodeWindow { content } => {
                if let Some(first_line) = content.line(0)
                    && !first_line.text.is_empty()
                {
                    first_line.text.to_string()
                } else {
                    "empty document".to_string()
                }
            }
            TabWindow::RepairGraph(graph) => "Repair graph".to_string(),
            TabWindow::MdpExplorer => "MDP".to_string(),
            _ => {
                todo!()
            }
        }
    }

    fn icon(&self, shared_state: &SharedState) -> Option<Handle> {
        match self {
            TabWindow::CodeWindow { .. } => Some(Handle::from_path(
                "repair-gui/resources/icons/prism_logo.png",
            )),
            TabWindow::RepairGraph(graph) => None,
            TabWindow::MdpExplorer => None,
            _ => {
                todo!()
            }
        }
    }

    fn update(&mut self, action: TabAction, shared_state: &mut SharedState) -> Task<TabAction> {
        match action {
            TabAction::EditCode(action) => {
                if let TabWindow::CodeWindow { content } = self {
                    content.perform(action);
                    Task::none()
                } else {
                    panic!("Tried to perform edit action on incorrect tab type");
                }
            }
            TabAction::RepairGraphMessage(message) => {
                if let TabWindow::RepairGraph(graph) = self {
                    graph.update(shared_state, message);
                    Task::none()
                } else {
                    panic!("Tried to perform repair graph action on incorrect tab type")
                }
            }
        }
    }

    fn view<'a>(&'a self, shared_state: &SharedState) -> Element<'a, TabAction> {
        match self {
            TabWindow::CodeWindow { content } => TextEditor::new(content)
                .on_action(TabAction::EditCode)
                .into(),
            TabWindow::MdpExplorer => text! {"mdp explorer"}.into(),
            TabWindow::RepairGraph(graph) => {
                graph.view(shared_state).map(TabAction::RepairGraphMessage)
            }
            _ => {
                todo!()
            }
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
