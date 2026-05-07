use crate::applied_repairs::{Fix, FixType};
use crate::prism_output::OutputVariableValues;
use prism_model::{
    Displayable, Expression, Identifier, Model, VariableInfo, VariableManager, VariableRange,
    VariableReference,
};
use prism_parser::Span;
use std::collections::HashMap;

pub enum RepairVariable {
    Bool,
    Integer { min: i64, max: i64 },
}

pub struct RepairCollection {
    first_variable_index: usize,
    repair_variables: Vec<RepairVariable>,
    pub repairs: Vec<Repair>,
    name_representatives: HashMap<String, usize>,
}

impl RepairCollection {
    pub fn new(existing_variable_count: usize) -> Self {
        Self {
            first_variable_index: existing_variable_count,
            repair_variables: Vec::new(),
            repairs: Vec::new(),
            name_representatives: HashMap::new(),
        }
    }
    // TODO: This currently expects that every returned kind is then added using `add_variable`, but
    //  this is never enforced
    pub fn request_to_reference(
        &mut self,
        name: Option<String>,
        kind: RepairKind<RepairVariable>,
    ) -> RepairKind<VariableReference> {
        if let Some(name) = name {
            if let Some(representative) = self.name_representatives.get(&name) {
                return kind.with_existing_variables(&self.repairs[*representative].kind);
            } else {
                self.name_representatives.insert(name, self.repairs.len());
            }
        }

        match kind {
            RepairKind::IntegerReplacement {
                variable,
                original_value,
            } => RepairKind::IntegerReplacement {
                variable: self.add_variable(variable),
                original_value,
            },
            RepairKind::Comparison {
                operator_variable,
                offset_variable,
                original_operator,
            } => RepairKind::Comparison {
                operator_variable: self.add_variable(operator_variable),
                offset_variable: self.add_variable(offset_variable),
                original_operator,
            },
            RepairKind::FunctionCall {
                function_type_variable,
                original_function,
                args,
            } => RepairKind::FunctionCall {
                function_type_variable: self.add_variable(function_type_variable),
                args,
                original_function,
            },
            RepairKind::Variable {
                variable_variable,
                offset_variable,
                original_variable,
            } => RepairKind::Variable {
                variable_variable: self.add_variable(variable_variable),
                offset_variable: self.add_variable(offset_variable),
                original_variable,
            },
        }
    }

    fn add_variable(&mut self, variable: RepairVariable) -> VariableReference {
        let reference =
            VariableReference::new(self.first_variable_index + self.repair_variables.len());
        self.repair_variables.push(variable);
        reference
    }

    pub fn add_repair(
        &mut self,
        span: Span,
        kind: RepairKind<VariableReference>,
        cost_function: CostFunction,
    ) {
        self.repairs.push(Repair {
            original_span: span,
            kind,
            cost_function,
        })
    }

    pub fn add_variables_to_prism(
        &mut self,
        model: &mut Model<
            (),
            Identifier<Span>,
            Expression<VariableReference, Span>,
            VariableReference,
            Span,
        >,
    ) {
        model.init_statements_to_init_block();
        let span = model.span.clone();
        for (index, variable) in self.repair_variables.drain(..).enumerate() {
            match variable {
                RepairVariable::Bool => {
                    model
                        .variable_manager
                        .add_variable(VariableInfo::new(
                            Identifier::new(format!("repair_var_{index}"), span.clone()).unwrap(),
                            VariableRange::Boolean { span: span.clone() },
                            false,
                            None,
                            span.clone(),
                        ))
                        .unwrap();
                }
                RepairVariable::Integer { min, max } => {
                    model
                        .variable_manager
                        .add_variable(VariableInfo::new(
                            Identifier::new(format!("repair_var_{index}"), span.clone()).unwrap(),
                            VariableRange::BoundedInt {
                                min: Expression::Int(min, span.clone()),
                                max: Expression::Int(max, span.clone()),
                                span: span.clone(),
                            },
                            false,
                            None,
                            span.clone(),
                        ))
                        .unwrap();
                }
            }
        }

        // We don't actually need to add any init constraints, as the variable bounds already
        //  enforce everything.
    }
}

pub struct Repair {
    pub original_span: Span,
    pub kind: RepairKind<VariableReference>,
    pub cost_function: CostFunction,
}

impl Repair {
    pub fn get_cost_and_fixes(
        &self,
        fixes: &mut Vec<Fix>,
        values: &OutputVariableValues,
        variable_manager: &VariableManager<Expression<VariableReference, Span>, Span>,
    ) -> f64 {
        match &self.kind {
            RepairKind::IntegerReplacement {
                variable,
                original_value,
            } => {
                let new_value = values.get_int(*variable);

                let costs = self.cost_function.get_cost(*original_value, new_value);

                let fix_type = if *original_value == new_value {
                    FixType::NoChange
                } else {
                    FixType::Fixed
                };
                fixes.push(Fix::new(
                    self.original_span.clone(),
                    new_value.to_string(),
                    fix_type,
                ));

                costs
            }
            RepairKind::Comparison { .. } => {
                todo!()
            }
            RepairKind::FunctionCall {
                function_type_variable,
                args,
                original_function,
            } => {
                let new_value = values.get_int(*function_type_variable);

                let costs = self
                    .cost_function
                    .get_cost(original_function.to_integer(), new_value);

                let fix_type = if original_function.to_integer() == new_value {
                    FixType::NoChange
                } else {
                    FixType::Fixed
                };
                let new_name = match new_value {
                    0 => "min",
                    1 => "max",
                    _ => unreachable!(),
                };
                fixes.push(Fix::new(
                    self.original_span.clone(),
                    format!(
                        "{}({})",
                        new_name,
                        args.iter()
                            .map(|a| a.displayable(variable_manager).to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    fix_type,
                ));

                costs
            }
            RepairKind::Variable { .. } => {
                todo!()
            }
        }
    }
}

pub enum ComparisonOperator {
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
}

pub enum FunctionKind {
    Min,
    Max,
}

impl FunctionKind {
    pub fn to_integer(&self) -> i64 {
        match self {
            Self::Min => 0,
            Self::Max => 1,
        }
    }
}

pub enum RepairKind<V> {
    IntegerReplacement {
        variable: V,
        original_value: i64,
    },
    Comparison {
        operator_variable: V,
        offset_variable: V,
        original_operator: ComparisonOperator,
    },
    FunctionCall {
        function_type_variable: V,
        args: Vec<Expression<VariableReference, Span>>,
        original_function: FunctionKind,
    },
    Variable {
        variable_variable: V,
        offset_variable: V,
        original_variable: VariableReference,
    },
}

impl<V> RepairKind<V> {
    pub fn with_existing_variables(
        self,
        other: &RepairKind<VariableReference>,
    ) -> RepairKind<VariableReference> {
        match self {
            RepairKind::IntegerReplacement { original_value, .. } => {
                if let RepairKind::IntegerReplacement { variable, .. } = other {
                    RepairKind::IntegerReplacement {
                        variable: variable.clone(),
                        original_value,
                    }
                } else {
                    panic!("Cannot take existing variables from incompatible `RepairKind`.")
                }
            }
            RepairKind::Comparison {
                original_operator, ..
            } => {
                if let RepairKind::Comparison {
                    operator_variable,
                    offset_variable,
                    ..
                } = other
                {
                    RepairKind::Comparison {
                        operator_variable: operator_variable.clone(),
                        offset_variable: offset_variable.clone(),
                        original_operator,
                    }
                } else {
                    panic!("Cannot take existing variables from incompatible `RepairKind`.")
                }
            }
            RepairKind::FunctionCall {
                original_function,
                args,
                ..
            } => {
                if let RepairKind::FunctionCall {
                    function_type_variable,
                    ..
                } = other
                {
                    RepairKind::FunctionCall {
                        function_type_variable: function_type_variable.clone(),
                        original_function,
                        args,
                    }
                } else {
                    panic!("Cannot take existing variables from incompatible `RepairKind`.")
                }
            }
            RepairKind::Variable {
                original_variable, ..
            } => {
                if let RepairKind::Variable {
                    variable_variable,
                    offset_variable,
                    ..
                } = other
                {
                    RepairKind::Variable {
                        original_variable,
                        variable_variable: variable_variable.clone(),
                        offset_variable: offset_variable.clone(),
                    }
                } else {
                    panic!("Cannot take existing variables from incompatible `RepairKind`.")
                }
            }
        }
    }
}

pub enum CostFunction {
    Uniform { costs: f64 },
    Linear { factor: f64 },
}

impl CostFunction {
    pub fn from_expression(expression: &Expression<VariableReference, Span>) -> Self {
        match expression {
            Expression::Function(identifier, args, span) => {
                fn arg_to_float(arg: &Expression<VariableReference, Span>) -> f64 {
                    match arg {
                        Expression::Int(val, _) => *val as f64,
                        Expression::Float(val, _) => *val,
                        _ => panic!("Invalid value for repair costs: {:?}", arg),
                    }
                }
                match identifier.name.as_str() {
                    "uniform_costs" => Self::Uniform {
                        // TODO: The list of names must be matched in repairs/mod.rs, which is unfortunate
                        costs: arg_to_float(&args[0]),
                    },
                    "linear_costs" => Self::Linear {
                        factor: arg_to_float(&args[0]),
                    },
                    name => panic!("Invalid repair method: {}", name),
                }
            }
            _ => panic!("Invalid cost specifier for repair function"),
        }
    }

    pub fn get_cost(&self, original_value: i64, new_value: i64) -> f64 {
        match self {
            CostFunction::Uniform { costs } => {
                if original_value != new_value {
                    *costs
                } else {
                    0.0
                }
            }
            CostFunction::Linear { factor } => (original_value - new_value).abs() as f64 * factor,
        }
    }
}
