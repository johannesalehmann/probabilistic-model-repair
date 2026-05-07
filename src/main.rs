mod applied_repairs;
mod context;
mod html_engine;
mod input;
mod prism;
mod prism_output;
mod repairs;

use crate::applied_repairs::{AppliedRepairCollection, FixType};
use crate::html_engine::{RepairKind, RepairedSpan};

fn main() {
    let model_with_context = input::get_model("models/racetrack/model");
    let filter_property = format!(
        "\"Filtered\": filter(print, {}, \"init\");",
        model_with_context.specification
    );
    let model_source = model_with_context.model_source;
    let mut model = model_with_context.model;

    let variable_ranges = context::VariableRanges::from_model(&model);

    let mut repair_collection = repairs::wire_up_repairs(&mut model, &variable_ranges);
    repair_collection.add_variables_to_prism(&mut model);

    let ref_to_prism = prism_output::VariableReferenceToPrismIndex::from_model(&model);
    let model_string = model.to_string();
    let feasible_combinations = prism::call_prism(&model_string, &filter_property);

    let mut applied_repairs = AppliedRepairCollection::from_feasible_combinations(
        feasible_combinations,
        &ref_to_prism,
        &repair_collection,
        &model.variable_manager,
    );
    applied_repairs.sort();

    let mut repair_document = html_engine::RepairOutput::new(model_source.clone());
    let mut base_tab = html_engine::Repair::new_base();
    for repair in &repair_collection.repairs {
        base_tab.add_span(RepairedSpan::new(
            repair.original_span.clone(),
            model_source[repair.original_span.into_range()].to_string(),
            RepairKind::ToRepair,
        ))
    }
    repair_document.add_repair(base_tab);

    for applied_repair in applied_repairs.applied_repairs {
        let mut repair_tab = html_engine::Repair::new_repair(applied_repair.total_cost);
        for fix in applied_repair.fixes {
            repair_tab.add_span(RepairedSpan::new(
                fix.location,
                fix.replace_by,
                match fix.fix_type {
                    FixType::Fixed => RepairKind::Fix,
                    FixType::NoChange => RepairKind::Unchanged,
                },
            ))
        }

        repair_document.add_repair(repair_tab);
    }

    std::fs::write("repairs.html", repair_document.to_html()).unwrap();
}
