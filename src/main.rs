use crate::repair_graph::PropertyCollection;
use crate::repair_problem::{RepairProblemDescription, StepResult};
use prism_parser::ErrorWithSource;

mod model_manipulation;
mod preprocessing;
mod prism_runner;
mod repair_graph;
mod repair_problem;
mod task_graph;
mod tasks;

fn main() {
    let sources = [
        // (
        // "models/toy_synthesis/model.prism",
        // "models/toy_synthesis/model.props",
        //),
        (
            "models/synthesis_input_variable/model.prism",
            "models/synthesis_input_variable/model.props",
        ),
    ];

    for (model, props) in sources {
        match get_description(model, props) {
            Ok(description) => {
                let mut task = description.build();
                loop {
                    match task.step() {
                        StepResult::Done { model, properties } => {
                            std::fs::write("result.prism", model.to_string()).unwrap();
                            println!(
                                "Repair completed successfully. Final model written to `result.prism`."
                            );
                            break;
                        }
                        StepResult::MoreToDo => {}
                        StepResult::NoMoreTasks => {
                            println!("No more executable tasks");
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                err.print_error();
            }
        }
    }
    println!("Repair tool finished");
}

fn get_description<'a, 'b, 'c>(
    model: &'b str,
    props: &'c str,
) -> Result<RepairProblemDescription, DescriptionCreationError<'a>> {
    let model_source =
        std::fs::read_to_string(model).map_err(DescriptionCreationError::ModelFileIoError)?;
    let properties_source =
        std::fs::read_to_string(props).map_err(DescriptionCreationError::PropertyFileIoError)?;
    let properties = properties_source
        .trim()
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();
    let cloned_source = model_source.clone();
    let result = prism_parser::parse_model_and_props(model_source.as_str(), &properties[..])
        .all_ok()
        .map_err(|errors| {
            let errors = errors
                .into_iter()
                .map(|e| ErrorWithSource {
                    source: e.source,
                    error: e.error.into_owned(),
                })
                .collect();
            DescriptionCreationError::ParserErrors {
                model_source: cloned_source,
                property_sources: properties.iter().map(|p| p.to_string()).collect(),
                errors,
            }
        })?;
    let property_collection = PropertyCollection::new(result.properties);
    Ok(RepairProblemDescription::new(
        result.model,
        property_collection,
    ))
}

enum DescriptionCreationError<'a> {
    ModelFileIoError(std::io::Error),
    PropertyFileIoError(std::io::Error),
    ParserErrors {
        model_source: String,
        property_sources: Vec<String>,
        errors: Vec<ErrorWithSource<'a>>,
    },
}

impl<'a> DescriptionCreationError<'a> {
    pub fn print_error(&self) {
        match self {
            DescriptionCreationError::ModelFileIoError(io) => {
                println!("Error reading model file: {}", io)
            }
            DescriptionCreationError::PropertyFileIoError(io) => {
                println!("Error reading property file: {}", io)
            }
            DescriptionCreationError::ParserErrors {
                model_source,
                property_sources: error_sources,
                errors,
            } => {
                for err in errors {
                    let (name, source) = match err.source {
                        prism_parser::ErrorSource::Model => {
                            ("model file".to_string(), model_source.as_str())
                        }
                        prism_parser::ErrorSource::Property { index } => {
                            (format!("property {index}"), error_sources[index].as_str())
                        }
                    };
                    err.error.clone().print(Some(&name), source)
                }
            }
        }
    }
}
