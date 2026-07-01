use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::sync::mpsc;

pub struct MainToolRunner {
    calls: HashMap<(usize, usize), ToolCall>,
    update_sender: mpsc::Sender<ToolUpdate>,
    child_counter: usize,
}

impl MainToolRunner {
    pub fn new() -> (Self, mpsc::Receiver<ToolUpdate>) {
        let (update_sender, update_receiver) = mpsc::channel(128);
        (
            Self {
                calls: HashMap::new(),
                update_sender,
                child_counter: 0,
            },
            update_receiver,
        )
    }

    pub fn get_tool_runner(&mut self, model_index: usize, task_index: usize) -> ToolRunner {
        let id = self.child_counter;
        self.child_counter += 1;
        ToolRunner {
            update_sender: self.update_sender.clone(),
            task_id: (model_index, task_index),
            id,
            call_counter: 0,
            file_counter: 0,
        }
    }

    pub fn process_update(&mut self, update: ToolUpdate) {
        match update {
            ToolUpdate::Started {
                task_id,
                call_id,
                name,
                arguments,
            } => {
                self.calls.insert(
                    call_id,
                    ToolCall {
                        task_id,
                        name,
                        arguments,
                        output: None,
                    },
                );
            }
            ToolUpdate::Completed { call_id, output } => {
                self.calls.get_mut(&call_id).unwrap().output = Some(output);
            }
        }
    }
}

pub enum ToolUpdate {
    Started {
        task_id: (usize, usize),
        call_id: (usize, usize),
        name: String,
        arguments: Vec<OsString>,
    },
    Completed {
        call_id: (usize, usize),
        output: String,
    },
}

pub struct ToolCall {
    task_id: (usize, usize),
    name: String,
    arguments: Vec<OsString>,
    output: Option<String>,
}

pub struct ToolRunner {
    update_sender: mpsc::Sender<ToolUpdate>,
    task_id: (usize, usize),
    id: usize,
    call_counter: usize,
    file_counter: usize,
}

impl ToolRunner {
    pub fn temp_file(&mut self, file_type: &str) -> PathBuf {
        if !std::fs::exists("temp/").unwrap() {
            std::fs::create_dir("temp/").unwrap();
        }
        let directory =
            Path::new("temp/").join(format!("task_{}_{}", self.task_id.0, self.task_id.1));
        if !std::fs::exists(&directory).unwrap() {
            std::fs::create_dir(&directory).unwrap();
        }
        let joiner = if file_type.starts_with(".") { "" } else { "." };
        let file = format!("{}{}{}", self.file_counter, joiner, file_type);
        self.file_counter += 1;
        directory.join(file)
    }

    pub async fn run_tool<S: Into<String>>(
        &mut self,
        name: S,
        arguments: Vec<OsString>,
    ) -> Option<String> {
        let call_id = (self.id, self.call_counter);
        self.call_counter += 1;
        let name = name.into();
        let start_updated = ToolUpdate::Started {
            task_id: self.task_id,
            call_id,
            name: name.clone(),
            arguments: arguments.clone(),
        };
        self.update_sender.send(start_updated).await.unwrap();

        // TODO: Handle failures without panicking!
        let mut result = tokio::process::Command::new(name)
            .args(&arguments[..])
            .stdout(Stdio::piped())
            .output()
            .await;

        let output = match result {
            Ok(output) => output,
            Err(err) => panic!("Could not read `prism` output: {}", err),
        };

        let stdout = match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(err) => panic!("`prism` output is not valid utf8: {}", err),
        };

        let end_update = ToolUpdate::Completed {
            call_id,
            output: stdout.clone(),
        };

        self.update_sender.send(end_update).await.unwrap();

        Some(stdout)
    }
}
