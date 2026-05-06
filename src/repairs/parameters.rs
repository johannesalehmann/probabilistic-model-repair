use crate::repairs::CostFunction;
use prism_model::{Expression, VariableReference};
use prism_parser::Span;

pub enum Visibility {
    Sees(Vec<VariableReference>),
    Hidden(Vec<VariableReference>),
}

pub struct RepairParameters {
    pub grouped: Option<String>,
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
    pub costs: Option<CostFunction>,
    pub visibility: Option<Visibility>,
}

impl RepairParameters {
    pub fn from_function_arguments(arguments: &[Expression<VariableReference, Span>]) -> Self {
        let mut parameters = Self {
            grouped: None,
            minimum: None,
            maximum: None,
            costs: None,
            visibility: None,
        };
        for argument in arguments {
            if let Expression::Function(name, args, _) = argument {
                match name.name.as_str() {
                    "grouped" => {
                        assert_eq!(args.len(), 1, "`grouped` must have exactly one argument");
                        if let Expression::Function(name, args, _) = &args[0] {
                            assert_eq!(args.len(), 0);
                            if parameters.grouped.is_some() {
                                panic!("Cannot specify grouping twice!");
                            }
                            parameters.grouped = Some(name.name.clone());
                        } else {
                            panic!(
                                "The grouping name for `grouped` must be given as a function call, e.g. `grouped(group_name()`"
                            )
                        }
                    }
                    "min" => {
                        assert_eq!(args.len(), 1, "`min` must have exactly one argument");
                        if let Expression::Int(val, _) = &args[0] {
                            if parameters.minimum.is_some() {
                                panic!("Cannot specify minimum twice");
                            }
                            parameters.minimum = Some(*val);
                        }
                    }
                    "max" => {
                        assert_eq!(args.len(), 1, "`max` must have exactly one argument");
                        if let Expression::Int(val, _) = &args[0] {
                            if parameters.minimum.is_some() {
                                panic!("Cannot specify maximum twice");
                            }
                            parameters.maximum = Some(*val);
                        }
                    }
                    "linear_costs" | "uniform_costs" => {
                        let costs = CostFunction::from_expression(argument);
                        if parameters.costs.is_some() {
                            panic!("Costs cannot be specified twice");
                        }
                        parameters.costs = Some(costs);
                    }
                    "sees" | "hidden" => {
                        let variables = args.iter().map(|a| {
                            if let Expression::VarOrConst(var, _) = a {
                                var.clone()
                            } else {
                                panic!("`sees` and `hidden` must contain a list of variable names as parameters")
                            }
                        }).collect::<Vec<_>>();
                        let visibility = if name.name.as_str() == "sees" {
                            Visibility::Sees(variables)
                        } else {
                            Visibility::Hidden(variables)
                        };
                        if parameters.visibility.is_some() {
                            panic!(
                                "Cannot specify visibility twice. `sees` and `hidden` are mutually exclusive."
                            )
                        }
                        parameters.visibility = Some(visibility);
                    }
                    name => panic!("Invalid repair parameter: `{name}(...)`"),
                }
            } else {
                panic!("All repair parameters must have form of a function call!");
            }
        }

        parameters
    }
}
