mod prism;

use prism_model::{
    Expression, Identifier, IdentityMapExpression, MapExpression, VariableInfo,
    VariableRange, VariableReference,
};
use prism_parser::Span;
use std::fs;

struct RepairModule {
    init_constraints: Vec<Expression<VariableReference, Span>>,
    variables: Vec<VariableInfo<Expression<VariableReference, Span>, Span>>,
    variable_counter: usize,
}

impl RepairModule {
    pub fn new(variable_counter: usize) -> Self {
        Self {
            init_constraints: Vec::new(),
            variables: Vec::new(),
            variable_counter,
        }
    }

    pub fn add_variable(
        &mut self,
        info: VariableInfo<Expression<VariableReference, Span>, Span>,
    ) -> VariableReference {
        let reference = VariableReference::new(self.variable_counter);
        self.variable_counter += 1;
        self.variables.push(info);
        reference
    }
}

fn main() {
    let model = prism_parser::parse_prism::<&str>(
        &fs::read_to_string("models/racetrack/model.prism").unwrap(),
        &[],
    );
    let specification = fs::read_to_string("models/racetrack/model.props").unwrap();

    if let Some(mut model) = model.model.output {
        let mut repair_module = RepairModule::new(model.variable_manager.variables.len());

        let mut variable_bounds = Vec::new();

        for variable in &model.variable_manager.variables {
            variable_bounds.push(match &variable.range {
                VariableRange::BoundedInt { min, max, .. } => {
                    let min = RepairVisitor::evaluate_const(min).int();
                    let max = RepairVisitor::evaluate_const(max).int();
                    if let (Some(min), Some(max)) = (min, max) {
                        PermissibleBounds::IntegerRange { min, max }
                    } else {
                        PermissibleBounds::Unknown
                    }
                }
                _ => PermissibleBounds::Unknown,
            })
        }

        for module in &mut model.modules.modules {
            for command in &mut module.commands {
                for update in &mut command.updates {
                    for assignment in &mut update.assignments {
                        fix_expression(
                            &mut assignment.value,
                            &RepairContext::Assignment {
                                variable: assignment.target,
                            },
                            &mut repair_module,
                            &variable_bounds,
                        );
                    }
                }
            }
        }

        model.init_statements_to_init_block();

        for variable in repair_module.variables {
            model.variable_manager.add_variable(variable).unwrap();
        }

        let mut init_conjunction = Expression::Bool(true, Span::splat(0));
        for init in repair_module.init_constraints {
            let temp = std::mem::replace(
                &mut init_conjunction,
                Expression::Bool(true, Span::splat(0)),
            );
            init_conjunction =
                Expression::Conjunction(Box::new(temp), Box::new(init), Span::splat(0));
        }

        match model.init_constraint {
            None => model.init_constraint = Some(init_conjunction),
            Some(init) => {
                model.init_constraint = Some(Expression::Conjunction(
                    Box::new(init),
                    Box::new(init_conjunction),
                    Span::splat(0),
                ))
            }
        }
        let model = model.to_string();
        let property = format!("\"Filtered\": filter(print, {}, \"init\");", specification);
        prism::call_prism(&model, &property);
    } else {
        println!("Failed to parse model!");
    }
}

enum RepairContext {
    InitialValue { variable: VariableReference },
    Guard,
    Probability,
    Assignment { variable: VariableReference },
}

fn fix_expression(
    expression: &mut Expression<VariableReference, Span>,
    context: &RepairContext,
    repair_module: &mut RepairModule,
    bounds: &Vec<PermissibleBounds>,
) {
    let mut repair_visitor = RepairVisitor::new(repair_module, bounds);
    let bounds = match context {
        RepairContext::InitialValue { variable } => bounds[variable.index],
        RepairContext::Guard => PermissibleBounds::Unknown,
        RepairContext::Probability => PermissibleBounds::FloatRange { min: 0.0, max: 1.0 },
        RepairContext::Assignment { variable } => bounds[variable.index],
    };
    repair_visitor.visit_expression(expression, bounds);
}

#[derive(Copy, Clone)]
enum PermissibleBounds {
    IntegerRange { min: i64, max: i64 },
    FloatRange { min: f64, max: f64 },
    Unknown,
}

impl PermissibleBounds {
    fn apply_integer_operation<F: Fn(i64) -> i64>(self, op: F) -> Self {
        match self {
            PermissibleBounds::IntegerRange { min, max } => {
                let min = op(min);
                let max = op(max);
                PermissibleBounds::IntegerRange {
                    min: min.min(max),
                    max: min.max(max),
                }
            }
            _ => PermissibleBounds::Unknown,
        }
    }
    fn apply_integer_operation_with_rounding<F: Fn(i64) -> f64>(self, op: F) -> Self {
        match self {
            PermissibleBounds::IntegerRange { min, max } => {
                let min = op(min);
                let max = op(max);
                let (min, max) = (min.min(max).ceil() as i64, min.max(max).floor() as i64);
                PermissibleBounds::IntegerRange { min, max }
            }
            _ => PermissibleBounds::Unknown,
        }
    }
    fn apply_numeric_operation<FI: Fn(i64) -> i64, FF: Fn(f64) -> f64>(
        self,
        op_int: FI,
        op_float: FF,
    ) -> Self {
        match self {
            PermissibleBounds::IntegerRange { min, max } => {
                let min = op_int(min);
                let max = op_int(max);
                PermissibleBounds::IntegerRange {
                    min: min.min(max),
                    max: min.max(max),
                }
            }
            PermissibleBounds::FloatRange { min, max } => {
                let min = op_float(min);
                let max = op_float(max);
                PermissibleBounds::FloatRange {
                    min: min.min(max),
                    max: min.max(max),
                }
            }

            PermissibleBounds::Unknown => PermissibleBounds::Unknown,
        }
    }
}

enum ConstEvaluationResult {
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

struct RepairVisitor<'a, 'b> {
    bounds: &'a Vec<PermissibleBounds>,
    repair_module: &'b mut RepairModule,
}

impl<'a, 'b> RepairVisitor<'a, 'b> {
    pub fn new(repair_module: &'b mut RepairModule, bounds: &'a Vec<PermissibleBounds>) -> Self {
        Self {
            repair_module,
            bounds,
        }
    }

    fn create_repair_variable(&mut self, min: i64, max: i64) -> VariableReference {
        let name = format!(
            "repair_variable_autogen_{}",
            self.repair_module.variable_counter
        );
        let reference = self.repair_module.add_variable(VariableInfo::new(
            Identifier::new(name.clone(), Span::splat(0)).unwrap(),
            VariableRange::BoundedInt {
                min: Expression::Int(min, Span::splat(0)),
                max: Expression::Int(max, Span::splat(0)),
                span: Span::splat(0),
            },
            false,
            None,
            Span::splat(0),
        ));
        self.repair_module.init_constraints.push(
            Expression::var_or_const(reference)
                .greater_or_equal(Expression::int(min))
                .and(Expression::var_or_const(reference).less_or_equal(Expression::int(max)))
                .map_span(&|_| Span::splat(0)),
        );
        reference
    }

    pub fn visit_expression(
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
            Expression::Function(name, args, _) => {
                if name.name == "repair" {
                    if let Expression::Int(val, _) = args[0] {
                        match bounds {
                            PermissibleBounds::IntegerRange { min, max } => {
                                let reference = self.create_repair_variable(min, max);
                                *expression = Expression::VarOrConst(reference, Span::splat(0));
                            }
                            PermissibleBounds::FloatRange { .. } => {
                                panic!(
                                    "Cannot repair integer because it needs to adhere to a floating-point range."
                                )
                            }
                            PermissibleBounds::Unknown => {
                                panic!("Cannot repair integer because its bounds are unknown");
                            }
                        }
                    } else {
                        println!("Cannot repair expression of this type");
                    }
                } else {
                    for arg in args {
                        self.visit_expression(arg, PermissibleBounds::Unknown)
                    }
                }
            }
            Expression::Minus(val, _) => {
                self.visit_expression(val, bounds.apply_numeric_operation(|v| -v, |v| -v))
            }
            Expression::Multiplication(val_1, val_2, _) => {
                // TODO: Properly handle floats here and in the other arithmetic operations
                let first_constant = Self::evaluate_const(val_1).int();
                let second_constant = Self::evaluate_const(val_2).int();
                match (first_constant, second_constant) {
                    (Some(v1), None) => self.visit_expression(
                        val_2,
                        bounds.apply_integer_operation_with_rounding(|v2| v2 as f64 / v1 as f64),
                    ),
                    (None, Some(v2)) => self.visit_expression(
                        val_1,
                        bounds.apply_integer_operation_with_rounding(|v1| v1 as f64 / v2 as f64),
                    ),
                    _ => {
                        self.visit_expression(val_1, PermissibleBounds::Unknown);
                        self.visit_expression(val_2, PermissibleBounds::Unknown);
                    }
                }
            }
            Expression::Division(val_1, val_2, _) => {
                // TODO: Handle bounds for division
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Addition(val_1, val_2, _) => {
                let first_constant = Self::evaluate_const(val_1).int();
                let second_constant = Self::evaluate_const(val_2).int();
                match (first_constant, second_constant) {
                    (Some(v1), None) => {
                        self.visit_expression(val_2, bounds.apply_integer_operation(|v2| v2 - v1))
                    }
                    (None, Some(v2)) => {
                        self.visit_expression(val_1, bounds.apply_integer_operation(|v1| v1 - v2))
                    }
                    _ => {
                        self.visit_expression(val_1, PermissibleBounds::Unknown);
                        self.visit_expression(val_2, PermissibleBounds::Unknown);
                    }
                }
            }
            Expression::Subtraction(val_1, val_2, _) => {
                let first_constant = Self::evaluate_const(val_1).int();
                let second_constant = Self::evaluate_const(val_2).int();
                match (first_constant, second_constant) {
                    (Some(v1), None) => {
                        self.visit_expression(val_2, bounds.apply_integer_operation(|v2| v1 - v2))
                    }
                    (None, Some(v2)) => {
                        self.visit_expression(val_1, bounds.apply_integer_operation(|v1| v1 + v2))
                    }
                    _ => {
                        self.visit_expression(val_1, PermissibleBounds::Unknown);
                        self.visit_expression(val_2, PermissibleBounds::Unknown);
                    }
                }
            }
            Expression::LessThan(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::LessOrEqual(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::GreaterThan(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::GreaterOrEqual(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Equals(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::NotEquals(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Negation(val_1, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
            }
            Expression::Conjunction(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Disjunction(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::IfAndOnlyIf(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Implies(val_1, val_2, _) => {
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
            Expression::Ternary(guard, val_1, val_2, _) => {
                self.visit_expression(guard, PermissibleBounds::Unknown);
                self.visit_expression(val_1, PermissibleBounds::Unknown);
                self.visit_expression(val_2, PermissibleBounds::Unknown);
            }
        }
    }

    pub fn evaluate_const(
        expression: &Expression<VariableReference, Span>,
    ) -> ConstEvaluationResult {
        // TODO: Support const-folding centrally in the expression library, instead of implementing it ad-hoc here.
        match expression {
            Expression::Int(val, _) => ConstEvaluationResult::Int(*val),
            Expression::Float(val, _) => ConstEvaluationResult::Float(*val),
            Expression::Bool(val, _) => ConstEvaluationResult::Bool(*val),
            Expression::VarOrConst(_, _) => ConstEvaluationResult::Variable,
            Expression::Label(_, _) => ConstEvaluationResult::Variable,
            Expression::Function(_, _, _) => ConstEvaluationResult::Variable,
            Expression::Minus(inner, _) => {
                let inner = Self::evaluate_const(inner);
                match inner {
                    ConstEvaluationResult::Int(val) => ConstEvaluationResult::Int(-val),
                    ConstEvaluationResult::Float(val) => ConstEvaluationResult::Float(val),
                    _ => ConstEvaluationResult::Variable,
                }
            }
            _ => ConstEvaluationResult::Variable,
        }
    }
}
