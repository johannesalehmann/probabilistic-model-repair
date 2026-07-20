use prism_parser::ErrorWithSource;
use std::path::{Path, PathBuf};

struct Paths {
    result: PathBuf,
    temp: PathBuf,
    model: String,
    properties: String,
}

impl Paths {
    fn search_directory(path: &str) -> Self {
        // TODO: Properly handle path concatenation
        let files = std::fs::read_dir(path)
            .unwrap_or_else(|e| panic!("Could not read directory {}: {}", path, e));

        let (mut model, mut properties) = (None, None);

        for file in files {
            let file = file.expect("Could not read file while exploring directory");
            if file.file_name() == "model.prism"
                || (model.is_none() && file.file_name().to_str().unwrap().ends_with(".prism"))
            {
                model = Some(file.path().to_str().unwrap().to_string());
            }
            if file.file_name() == "model.props"
                || (model.is_none() && file.file_name().to_str().unwrap().ends_with(".props"))
            {
                properties = Some(file.path().to_str().unwrap().to_string());
            }
        }

        if model.is_none() {
            panic!("Could not find model file (ending with suffix `.prism`) in directory {path}");
        }
        if properties.is_none() {
            panic!(
                "Could not find property file (ending with suffix `.props`) in directory {path}"
            );
        }

        let temp = Path::new(path).join("temp");
        if std::fs::exists(&temp).unwrap() {
            std::fs::remove_dir_all(&temp).unwrap();
        }
        std::fs::create_dir(&temp).unwrap();

        let result = Path::new(path).join("result.prism");

        Self {
            temp,
            model: model.unwrap(),
            properties: properties.unwrap(),
            result,
        }
    }
}

fn main() {
    let sources = [Paths::search_directory("models/synthesis_input_variable/")];

    for path in sources {
        println!("Repairing model `{}`", path.model);
        match get_description(&path) {
            Ok(description) => {
                let mut task = description.build();
                loop {
                    match task.step() {
                        StepResult::Done { model, properties } => {
                            let target = std::fs::write(&path.result, model.to_string()).unwrap();
                            println!(
                                "Repair completed successfully. Final model written to `{}`.",
                                path.result.to_str().unwrap()
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

fn get_description<'a>(
    paths: &Paths,
) -> Result<RepairProblemDescription, DescriptionCreationError<'a>> {
    let model_source = std::fs::read_to_string(&paths.model)
        .map_err(DescriptionCreationError::ModelFileIoError)?;
    let properties_source = std::fs::read_to_string(&paths.properties)
        .map_err(DescriptionCreationError::PropertyFileIoError)?;
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
                    source: e.source,{

                }
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
        paths.temp.clone(),
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
