use super::permissible_bounds::PermissibleBounds;
use prism_model::{Expression, Identifier, Model, VariableRange, VariableReference};
use prism_parser::Span;

pub struct VariableRanges {
    pub bounds: Vec<PermissibleBounds>,
}

impl VariableRanges {
    pub fn from_model(
        model: &Model<
            (),
            Identifier<Span>,
            Expression<VariableReference, Span>,
            VariableReference,
            Span,
        >,
    ) -> VariableRanges {
        let mut bounds = Vec::new();
        for variable in &model.variable_manager.variables {
            bounds.push(match &variable.range {
                VariableRange::BoundedInt { min, max, .. } => {
                    let min = crate::repairs::evaluate_const(min).int();
                    let max = crate::repairs::evaluate_const(max).int();
                    if let (Some(min), Some(max)) = (min, max) {
                        PermissibleBounds::IntegerRange { min, max }
                    } else {
                        PermissibleBounds::Unknown
                    }
                }
                _ => PermissibleBounds::Unknown,
            })
        }
        Self { bounds }
    }
}
