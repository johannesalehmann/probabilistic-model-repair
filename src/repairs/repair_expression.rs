pub use crate::context::PermissibleBounds;
use crate::context::VariableRanges;
use crate::repairs::CostFunction;
use crate::repairs::types::{RepairKind, RepairVariable};
use prism_model::{Expression, Identifier, VariableReference};
use prism_parser::Span;

pub struct Repairer<'a, 'b> {
    variable_ranges: &'a VariableRanges,
    repairs: &'b mut super::RepairCollection,
}

impl<'a, 'b> Repairer<'a, 'b> {
    pub fn new(
        variable_ranges: &'a VariableRanges,
        repairs: &'b mut super::RepairCollection,
    ) -> Self {
        Self {
            variable_ranges,
            repairs,
        }
    }

    fn process_repair(
        &mut self,
        args: &Vec<Expression<VariableReference, Span>>,
        span: &Span,
        bounds: PermissibleBounds,
    ) -> Expression<VariableReference, Span> {
        let parameters = super::parameters::RepairParameters::from_function_arguments(&args[1..]);
        let repair_kind = match &args[0] {
            Expression::Int(val, span) => {
                let repair = RepairKind::IntegerReplacement {
                    variable: RepairVariable::Integer {
                        min: bounds.min_int(),
                        max: bounds.max_int(),
                    },
                    original_value: *val,
                };
                self.repairs
                    .request_to_reference(parameters.grouped, repair)
            }
            _ => panic!("Cannot repair an expression of this type"),
        };

        match repair_kind {
            RepairKind::IntegerReplacement { variable, .. } => {
                let repair = Expression::VarOrConst(variable, span.clone());
                self.repairs.add_repair(
                    span.clone(),
                    repair_kind,
                    parameters
                        .costs
                        .unwrap_or_else(|| CostFunction::Linear { factor: 1.0 }),
                );
                repair
            }
            RepairKind::Comparison { .. } => {
                todo!();
            }
            RepairKind::FunctionCall { .. } => {
                todo!();
            }
            RepairKind::Variable { .. } => {
                todo!();
            }
        }
    }

    pub fn repair_expression(
        &mut self,
        expression: &mut Expression<VariableReference, Span>,
        bounds: PermissibleBounds,
    ) {
        match expression {
            Expression::Int(_, _) => {}
            Expression::Float(_, _) => {}
            Expression::Bool(_, _) => {}
            Expression::VarOrConst(_, _) => {}
            Expression::Label(_, _) => {}
            Expression::Function(name, args, span) => {
                if name.name == "repair" {
                    let repair = self.process_repair(args, span, bounds);
                    *expression = repair;
                } else {
                    for arg in args {
                        self.repair_expression(arg, PermissibleBounds::Unknown)
                    }
                }
            }
            Expression::Minus(val, _) => {
                self.repair_expression(val, bounds.apply_numeric_operation(|v| -v, |v| -v))
            }
            Expression::Multiplication(val_1, val_2, _) => {
                // TODO: Properly handle floats here and in the other arithmetic operations
                let first_constant = evaluate_const(val_1).int();
                let second_constant = evaluate_const(val_2).int();
                match (first_constant, second_constant) {
                    (Some(v1), None) => self.repair_expression(
                        val_2,
                        bounds.apply_integer_operation_with_rounding(|v2| v2 as f64 / v1 as f64),
                    ),
                    (None, Some(v2)) => self.repair_expression(
                        val_1,
                        bounds.apply_integer_operation_with_rounding(|v1| v1 as f64 / v2 as f64),
                    ),
                    _ => {
                        self.repair_expression(val_1, PermissibleBounds::Unknown);
                        self.repair_expression(val_2, PermissibleBounds::Unknown);
                    }
                }
            }
            Expression::Division(val_1, val_2, _) => {
                // TODO: Handle bounds for division
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Addition(val_1, val_2, _) => {
                let first_constant = evaluate_const(val_1).int();
                let second_constant = evaluate_const(val_2).int();
                match (first_constant, second_constant) {
                    (Some(v1), None) => {
                        self.repair_expression(val_2, bounds.apply_integer_operation(|v2| v2 - v1))
                    }
                    (None, Some(v2)) => {
                        self.repair_expression(val_1, bounds.apply_integer_operation(|v1| v1 - v2))
                    }
                    _ => {
                        self.repair_expression(val_1, PermissibleBounds::Unknown);
                        self.repair_expression(val_2, PermissibleBounds::Unknown);
                    }
                }
            }
            Expression::Subtraction(val_1, val_2, _) => {
                let first_constant = evaluate_const(val_1).int();
                let second_constant = evaluate_const(val_2).int();
                match (first_constant, second_constant) {
                    (Some(v1), None) => {
                        self.repair_expression(val_2, bounds.apply_integer_operation(|v2| v1 - v2))
                    }
                    (None, Some(v2)) => {
                        self.repair_expression(val_1, bounds.apply_integer_operation(|v1| v1 + v2))
                    }
                    _ => {
                        self.repair_expression(val_1, PermissibleBounds::Unknown);
                        self.repair_expression(val_2, PermissibleBounds::Unknown);
                    }
                }
            }
            Expression::LessThan(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::LessOrEqual(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::GreaterThan(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::GreaterOrEqual(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Equals(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::NotEquals(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Negation(val_1, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
            }
            Expression::Conjunction(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Disjunction(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::IfAndOnlyIf(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Implies(val_1, val_2, _) => {
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Ternary(guard, val_1, val_2, _) => {
                self.repair_expression(guard, PermissibleBounds::Unknown);
                self.repair_expression(val_1, PermissibleBounds::Unknown);
                self.repair_expression(val_2, PermissibleBounds::Unknown);
            }
        }
    }
}
pub fn evaluate_const(expression: &Expression<VariableReference, Span>) -> ConstEvaluationResult {
    // TODO: Support const-folding centrally in the expression library, instead of implementing it ad-hoc here.
    match expression {
        Expression::Int(val, _) => ConstEvaluationResult::Int(*val),
        Expression::Float(val, _) => ConstEvaluationResult::Float(*val),
        Expression::Bool(val, _) => ConstEvaluationResult::Bool(*val),
        Expression::VarOrConst(_, _) => ConstEvaluationResult::Variable,
        Expression::Label(_, _) => ConstEvaluationResult::Variable,
        Expression::Function(_, _, _) => ConstEvaluationResult::Variable,
        Expression::Minus(inner, _) => {
            let inner = evaluate_const(inner);
            match inner {
                ConstEvaluationResult::Int(val) => ConstEvaluationResult::Int(-val),
                ConstEvaluationResult::Float(val) => ConstEvaluationResult::Float(val),
                _ => ConstEvaluationResult::Variable,
            }
        }
        _ => ConstEvaluationResult::Variable,
    }
}
pub enum ConstEvaluationResult {
    Int(i64),
    Float(f64),
    Bool(bool),
    Variable,
}

impl ConstEvaluationResult {
    pub fn int(&self) -> Option<i64> {
        match self {
            Self::Int(val) => Some(*val),
            _ => None,
        }
    }
    pub fn float(&self) -> Option<f64> {
        match self {
            Self::Float(val) => Some(*val),
            _ => None,
        }
    }
    pub fn bool(&self) -> Option<bool> {
        match self {
            Self::Bool(val) => Some(*val),
            _ => None,
        }
    }
}
