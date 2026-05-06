mod parameters;
mod repair_expression;
mod types;

use crate::context::{PermissibleBounds, VariableRanges};
use prism_model::{Expression, Identifier, Model, VariableReference};
use prism_parser::Span;
use repair_expression::Repairer;
pub use repair_expression::evaluate_const;
pub use types::CostFunction;
pub use types::RepairCollection;

pub fn wire_up_repairs(
    model: &mut Model<
        (),
        Identifier<Span>,
        Expression<VariableReference, Span>,
        VariableReference,
        Span,
    >,
    variable_ranges: &VariableRanges,
) -> RepairCollection {
    let mut repairs = RepairCollection::new(model.variable_manager.variables.len());

    for (var_index, variable) in model.variable_manager.variables.iter_mut().enumerate() {
        if let Some(initial_value) = &mut variable.initial_value {
            let mut repairer = Repairer::new(variable_ranges, &mut repairs);
            repairer.repair_expression(initial_value, variable_ranges.bounds[var_index]);
        }
    }

    for module in &mut model.modules.modules {
        for command in &mut module.commands {
            let mut repairer = Repairer::new(variable_ranges, &mut repairs);
            repairer.repair_expression(&mut command.guard, PermissibleBounds::Unknown);

            // TODO: Figure out restricted ranges

            for update in &mut command.updates {
                // TODO: Probability repair probably requires separate handling
                let mut repairer = Repairer::new(variable_ranges, &mut repairs);
                repairer.repair_expression(&mut update.probability, PermissibleBounds::Unknown);

                for assignment in &mut update.assignments {
                    let mut repairer = Repairer::new(variable_ranges, &mut repairs);
                    repairer.repair_expression(
                        &mut assignment.value,
                        variable_ranges.bounds[assignment.target.index],
                    );
                }
            }
        }
    }

    repairs
}
