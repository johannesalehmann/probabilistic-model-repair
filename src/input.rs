use prism_model::{Expression, Identifier, Model, VariableReference};
use prism_parser::Span;
use std::fs;

pub struct ModelWithContext {
    pub model:
        Model<(), Identifier<Span>, Expression<VariableReference, Span>, VariableReference, Span>,
    pub model_source: String,
    pub specification: String,
}

pub fn get_model(base_path: &str) -> ModelWithContext {
    let model_source = fs::read_to_string(format!("{}.prism", base_path)).unwrap();
    let model = prism_parser::parse_prism::<&str>(&model_source, &[]);
    let specification = fs::read_to_string(format!("{}.props", base_path)).unwrap();

    if let Some(model) = model.model.output {
        ModelWithContext {
            model,
            model_source,
            specification,
        }
    } else {
        panic!("Failed to parse model.");
    }
}
