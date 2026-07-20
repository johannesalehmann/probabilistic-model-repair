use crate::repair_graph::{CheckingResult, CheckingResults, PrismModel, PropertyCollection};
use crate::tool_runner::ToolRunner;
use prism_model::Displayable;
use probabilistic_properties::{BoundOperator, NonDeterminismKind, StateFormula};

pub async fn check_properties(
    model: &PrismModel,
    properties: &PropertyCollection,
    tool_runner: &mut ToolRunner,
) -> CheckingResults {
    let model_source = model.to_string();
    let property_sources = properties
        .properties
        .iter()
        .map(|p| {
            let mut p = p.clone();
            if let probabilistic_properties::Query::StateFormula(StateFormula::ProbabilityBound {
                non_determinism,
                bound,
                path: _path,
            }) = &mut p
            {
                match (&non_determinism, bound.operator) {
                    (Some(NonDeterminismKind::Minimise), BoundOperator::GreaterThan)
                    | (Some(NonDeterminismKind::Minimise), BoundOperator::GreaterOrEqual) => {
                        *non_determinism = None;
                    }
                    (Some(NonDeterminismKind::Maximise), BoundOperator::LessThan)
                    | (Some(NonDeterminismKind::Maximise), BoundOperator::LessOrEqual) => {
                        *non_determinism = None;
                    }
                    (Some(NonDeterminismKind::Minimise), BoundOperator::LessThan) => {
                        panic!("PRISM cannot check a property of form `Pmin < t`")
                    }
                    (Some(NonDeterminismKind::Minimise), BoundOperator::LessOrEqual) => {
                        panic!("PRISM cannot check a property of form `Pmin <= t`")
                    }
                    (Some(NonDeterminismKind::Maximise), BoundOperator::GreaterThan) => {
                        panic!("PRISM cannot check a property of form `Pmax > t`")
                    }
                    (Some(NonDeterminismKind::Maximise), BoundOperator::GreaterOrEqual) => {
                        panic!("PRISM cannot check a property of form `Pmax > t`")
                    }

                    _ => (),
                }
            }
            p.map_i(&mut |i: prism_model::Expression| {
                i.displayable(&model.variable_manager).to_string()
            })
            .map_e(&mut |e: prism_model::Expression| {
                e.displayable(&model.variable_manager).to_string()
            })
            .map_f(&mut |f: prism_model::Expression| {
                f.displayable(&model.variable_manager).to_string()
            })
            .to_string()
        })
        .chain(std::iter::once("".to_string()))
        .collect::<Vec<_>>()
        .join(";\n");

    let file_name = tool_runner.temp_file("prism");
    let prop_name = tool_runner.temp_file("props");
    std::fs::write(&file_name, model_source).expect("Failed to write temporary file");
    std::fs::write(&prop_name, property_sources).expect("Failed to write temporary file");

    let stdout = tool_runner
        .run_tool(
            "prism",
            vec![file_name.into_os_string(), prop_name.into_os_string()],
        )
        .await
        .unwrap();

    let mut results = stdout
        .split("---------------------------------------------------------------------")
        .collect::<Vec<_>>();
    // If there are warnings, PRISM produces an extra "Note, There were n warnings during computation" section, which we remove in the following"
    if results
        .last()
        .map(|r| r.trim().starts_with("Note: There"))
        .unwrap_or(false)
    {
        results.remove(results.len() - 1);
    }
    if results.len() != properties.properties.len() + 1 {
        panic!(
            "PRISM output did not have the right number of sections delineated by dashed lines (expected {}, found {})\n\n:{stdout}",
            properties.properties.len() + 1,
            results.len()
        );
    }

    let mut res = CheckingResults::new();
    for i in results.len() - properties.properties.len()..results.len() {
        let output = results[i];
        let last_result = output.rfind("Result: ").unwrap_or_else(|| {
            panic!("Did not find `Result: ` in PRISM output for property {i}:\n\n{output}")
        });
        let after_last_result = &output[last_result + "Result: ".len()..].trim();
        let isolated = after_last_result
            .split_once(" ")
            .map(|(a, _)| a)
            .unwrap_or(after_last_result);
        let parsed =         match isolated {
            "true" => CheckingResult::Bool(true),
            "false" => CheckingResult::Bool(false),
            val => CheckingResult::Float(val.parse().unwrap_or_else(|_| panic!("Could not parse {val} as boolean or floating-point number ({val} is the result returned by PRISM for property {i}")))
        };
        res.results.push(parsed);
    }

    res
}
