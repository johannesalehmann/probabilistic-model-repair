use iced::advanced::image::Handle;
use iced::advanced::text::highlighter::PlainText;
use iced::futures::{AsyncBufReadExt, StreamExt};
use iced::widget::text::Highlighter;
use iced::widget::{TextEditor, text_editor};
use iced::{Element, Task};
use std::fmt::Debug;
use tabbed_workspace::TabbedWorkspace;

fn main() {
    iced::run(Window::update, Window::view).expect("Failed to run UI");
}

struct Window {
    workspace: TabbedWorkspace<TabWindow>,
}

impl Default for Window {
    fn default() -> Self {
        let mut workspace = TabbedWorkspace::new();
        workspace.open_window(TabWindow::code_window());
        workspace.open_window(TabWindow::code_window());
        workspace.open_window(TabWindow::code_window());
        workspace.open_window(TabWindow::MdpExplorer);
        workspace.open_window(TabWindow::RepairGraph);

        Self { workspace }
    }
}

impl Window {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Workspace(msg) => {
                return self.workspace.update(msg).map(Message::Workspace);
            }
        }
        Task::none()
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        self.workspace.view(Message::Workspace)
    }
}

enum Message {
    Workspace(tabbed_workspace::Message<TabAction>),
}

enum TabWindow {
    CodeWindow { content: text_editor::Content },
    RepairGraph,
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
}

impl tabbed_workspace::Window for TabWindow {
    type TabAction = TabAction;

    fn title(&self) -> String {
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
            TabWindow::RepairGraph => "Repair graph".to_string(),
            TabWindow::MdpExplorer => "MDP".to_string(),
        }
    }

    fn icon(&self) -> Option<Handle> {
        match self {
            TabWindow::CodeWindow { .. } => Some(Handle::from_path(
                "repair-gui/resources/icons/prism_logo.png",
            )),
            TabWindow::RepairGraph => None,
            TabWindow::MdpExplorer => None,
        }
    }

    fn update(&mut self, action: TabAction) {
        match action {
            TabAction::EditCode(action) => {
                if let TabWindow::CodeWindow { content } = self {
                    content.perform(action)
                } else {
                    panic!("Tried to perform edit action on incorrect tab type");
                }
            }
        }
    }

    fn view<'a>(&'a self) -> Element<'a, TabAction> {
        match self {
            TabWindow::CodeWindow { content } => TextEditor::new(content)
                .on_action(TabAction::EditCode)
                .into(),
            TabWindow::RepairGraph => iced::widget::text! {"Repair graph"}.into(),
            TabWindow::MdpExplorer => iced::widget::text! {"Mdp explorer"}.into(),
        }
    }
}
