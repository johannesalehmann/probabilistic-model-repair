use std::collections::HashMap;
use std::ffi::OsString;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::sync::mpsc;

pub struct MainToolRunner {
    log_entries: HashMap<(usize, usize), LogEntry>,
    entries_of_task: HashMap<(usize, usize), Vec<(usize, usize)>>,
    update_sender: mpsc::Sender<LogUpdate>,
    child_counter: usize,
}

impl MainToolRunner {
    pub fn new() -> (Self, mpsc::Receiver<LogUpdate>) {
        let (update_sender, update_receiver) = mpsc::channel(128);
        (
            Self {
                log_entries: HashMap::new(),
                entries_of_task: HashMap::new(),
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
            update_counter: 0,
            file_counter: 0,
        }
    }

    fn add_entry_for_task(&mut self, task_id: (usize, usize), call_id: (usize, usize)) {
        if let Some(entries) = self.entries_of_task.get_mut(&task_id) {
            entries.push(call_id);
        } else {
            self.entries_of_task.insert(task_id, vec![call_id]);
        }
    }

    pub fn entries_for_task(&self, task_id: (usize, usize)) -> &[(usize, usize)] {
        self.entries_of_task
            .get(&task_id)
            .map(|e| &e[..])
            .unwrap_or(&[])
    }

    pub fn entry(&self, entry_id: (usize, usize)) -> &LogEntry {
        self.log_entries.get(&entry_id).unwrap()
    }

    pub fn process_update(&mut self, update: LogUpdate) {
        match update {
            LogUpdate::Started {
                task_id,
                update_id: call_id,
                details,
            } => {
                let details = match details {
                    StartedDetails::ToolCall {
                        tool_name,
                        arguments,
                    } => LogDetails::ToolCall {
                        tool_name,
                        arguments,
                        output: None,
                    },
                    StartedDetails::Section { name, units } => LogDetails::Section {
                        name,
                        total_units: units,
                        completed_units: units.map(|_| 0),
                        final_status: None,
                    },
                };

                self.log_entries.insert(
                    call_id,
                    LogEntry {
                        task_id,
                        details,
                        start_time: Instant::now(),
                        end_time: None,
                    },
                );
                self.add_entry_for_task(task_id, call_id);
            }
            LogUpdate::Changed {
                update_id: call_id,
                details,
            } => {
                let call = self.log_entries.get_mut(&call_id).unwrap();
                match details {
                    ChangedDetails::SetUnitsFinished { units_finished } => {
                        if let LogDetails::Section {
                            completed_units, ..
                        } = &mut call.details
                        {
                            if let Some(units) = completed_units {
                                *units = units_finished;
                            }
                        } else {
                            panic!("Log update does not match the type of the log entry");
                        }
                    }
                    ChangedDetails::FinishedToolCall { output: new_output } => {
                        call.end_time = Some(Instant::now());
                        if let LogDetails::ToolCall { output, .. } = &mut call.details {
                            *output = Some(new_output);
                        } else {
                            panic!("Log update does not match the type of the log entry");
                        }
                    }
                    ChangedDetails::FinishedSection { status } => {
                        call.end_time = Some(Instant::now());
                        if let LogDetails::Section { final_status, .. } = &mut call.details {
                            *final_status = Some(status);
                        } else {
                            panic!("Log update does not match the type of the log entry");
                        }
                    }
                }
            }
        }
    }
}

pub enum LogUpdate {
    Started {
        task_id: (usize, usize),
        update_id: (usize, usize),
        details: StartedDetails,
    },
    Changed {
        update_id: (usize, usize),
        details: ChangedDetails,
    },
}

pub enum StartedDetails {
    ToolCall {
        tool_name: String,
        arguments: Vec<OsString>,
    },
    Section {
        name: String,
        units: Option<usize>,
    },
}

pub enum ChangedDetails {
    SetUnitsFinished { units_finished: usize },
    FinishedToolCall { output: String },
    FinishedSection { status: Status },
}

pub struct LogEntry {
    pub task_id: (usize, usize),
    pub details: LogDetails,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
}

pub enum LogDetails {
    ToolCall {
        tool_name: String,
        arguments: Vec<OsString>,
        output: Option<String>,
    },
    Section {
        name: String,
        total_units: Option<usize>,
        completed_units: Option<usize>,
        final_status: Option<Status>,
    },
}

pub struct Status {
    pub kind: StatusKind,
    pub details: Option<String>,
}

pub enum StatusKind {
    Success,
    Failure,
    Unknown,
}

pub struct ToolRunner {
    update_sender: mpsc::Sender<LogUpdate>,
    task_id: (usize, usize),
    id: usize,
    update_counter: usize,
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

    fn next_update_id(&mut self) -> (usize, usize) {
        let update_id = (self.id, self.update_counter);
        self.update_counter += 1;
        update_id
    }

    pub async fn run_tool<S: Into<String>, OS: Into<OsString>, VOS: Into<Vec<OS>>>(
        &mut self,
        name: S,
        arguments: VOS,
    ) -> Option<String> {
        let arguments: Vec<OsString> = arguments.into().into_iter().map(|s| s.into()).collect();

        let update_id = self.next_update_id();
        let name = name.into();
        self.update_sender
            .send(LogUpdate::Started {
                task_id: self.task_id,
                update_id,
                details: StartedDetails::ToolCall {
                    tool_name: name.clone(),
                    arguments: arguments.clone(),
                },
            })
            .await
            .unwrap();

        // TODO: Handle failures without panicking!
        let result = tokio::process::Command::new(name)
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

        self.update_sender
            .send(LogUpdate::Changed {
                update_id,
                details: ChangedDetails::FinishedToolCall {
                    output: stdout.clone(),
                },
            })
            .await
            .unwrap();

        Some(stdout)
    }

    pub async fn start_section(&mut self, name: String) -> Section<()> {
        self.start_section_internal(name, None).await
    }
    pub async fn start_section_with_units(&mut self, name: String, units: usize) -> Section<usize> {
        self.start_section_internal(name, Some(units)).await
    }
    async fn start_section_internal<T: Default>(
        &mut self,
        name: String,
        units: Option<usize>,
    ) -> Section<T> {
        let call_id = self.next_update_id();
        self.update_sender
            .send(LogUpdate::Started {
                task_id: self.task_id,
                update_id: call_id,
                details: StartedDetails::Section { name, units },
            })
            .await
            .unwrap();

        Section {
            log_id: call_id,
            sender: self.update_sender.clone(),
            section_type: Default::default(),
        }
    }
}

pub struct Section<U> {
    log_id: (usize, usize),
    sender: mpsc::Sender<LogUpdate>,
    section_type: PhantomData<U>,
}

impl<U> Section<U> {
    pub async fn finish_success(self) {
        self.finish_internal(Status {
            kind: StatusKind::Success,
            details: None,
        })
        .await
    }
    pub async fn finish_success_with_text<S: Into<String>>(self, details: S) {
        self.finish_internal(Status {
            kind: StatusKind::Success,
            details: Some(details.into()),
        })
        .await
    }

    pub async fn finish_unknown(self) {
        self.finish_internal(Status {
            kind: StatusKind::Unknown,
            details: None,
        })
        .await
    }
    pub async fn finish_unknown_with_text<S: Into<String>>(self, details: S) {
        self.finish_internal(Status {
            kind: StatusKind::Unknown,
            details: Some(details.into()),
        })
        .await
    }

    pub async fn finish_failure(self) {
        self.finish_internal(Status {
            kind: StatusKind::Failure,
            details: None,
        })
        .await
    }
    pub async fn finish_failure_with_text<S: Into<String>>(self, details: S) {
        self.finish_internal(Status {
            kind: StatusKind::Failure,
            details: Some(details.into()),
        })
        .await
    }

    async fn finish_internal(self, status: Status) {
        self.sender
            .send(LogUpdate::Changed {
                update_id: self.log_id,
                details: ChangedDetails::FinishedSection { status },
            })
            .await
            .unwrap();
    }
}

impl Section<usize> {
    pub async fn set_unit_count(&self, units_finished: usize) {
        self.sender
            .send(LogUpdate::Changed {
                update_id: self.log_id,
                details: ChangedDetails::SetUnitsFinished { units_finished },
            })
            .await
            .unwrap();
    }
}
