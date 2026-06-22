use iced::{Element, Task};
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
        workspace.open_window(TabWindow::CodeWindow);
        workspace.open_window(TabWindow::CodeWindow);
        workspace.open_window(TabWindow::CodeWindow);
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
    Workspace(tabbed_workspace::Message),
}

enum TabWindow {
    CodeWindow,
    RepairGraph,
    MdpExplorer,
}

impl tabbed_workspace::Window for TabWindow {
    fn title(&self) -> String {
        match self {
            TabWindow::CodeWindow => "Code".to_string(),
            TabWindow::RepairGraph => "Repair graph".to_string(),
            TabWindow::MdpExplorer => "MDP".to_string(),
        }
    }

    fn view<'a, Msg: 'a>(&'a self) -> Element<'a, Msg> {
        match self {
            TabWindow::CodeWindow => iced::widget::text! {"Code window"}.into(),
            TabWindow::RepairGraph => iced::widget::text! {"Repair graph"}.into(),
            TabWindow::MdpExplorer => iced::widget::text! {"Mdp explorer"}.into(),
        }
    }
}
