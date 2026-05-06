use prism_model::{Expression, VariableReference};
use prism_parser::Span;
use std::collections::HashMap;

struct VariablesConstraints {
    variables: HashMap<usize, VariableConstraints>,
}

impl VariablesConstraints {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn add_constraint(&mut self, variable: VariableReference, constraint: Constraint) {
        if let Some(variable) = self.variables.get_mut(&variable.index) {
            variable.constraints.push(constraint);
        } else {
            self.variables.insert(
                variable.index,
                VariableConstraints::with_constraint(constraint),
            );
        }
    }
}

struct VariableConstraints {
    constraints: Vec<Constraint>,
}

impl VariableConstraints {
    pub fn with_constraint(constraint: Constraint) -> Self {
        Self {
            constraints: vec![constraint],
        }
    }
}

enum Constraint {
    LessThan(i64),
    LessOrEqual(i64),
    GreaterThan(i64),
    GreaterOrEqual(i64),
    Equals(i64),
}

pub struct VariableBounds {
    pub lower_bound: Option<i64>,
    pub upper_bound: Option<i64>,
}

impl VariableBounds {
    fn from_variable_constraints(constraints: &VariableConstraints) -> Self {
        let mut lower_bound = i64::MIN;
        let mut upper_bound = i64::MAX;

        for constraint in &constraints.constraints {
            match constraint {
                Constraint::LessThan(val) => {
                    upper_bound = upper_bound.min(val - 1);
                }
                Constraint::LessOrEqual(val) => {
                    upper_bound = upper_bound.min(*val);
                }
                Constraint::GreaterThan(val) => {
                    lower_bound = lower_bound.max(val + 1);
                }
                Constraint::GreaterOrEqual(val) => {
                    lower_bound = lower_bound.max(*val);
                }
                Constraint::Equals(val) => {
                    // TODO: Properly handle case where additional constraints are placed that
                    //  contradict the equals constraint
                    lower_bound = *val;
                    upper_bound = *val;
                }
            }
        }

        let lower_bound = if lower_bound == i64::MIN {
            None
        } else {
            Some(lower_bound)
        };
        let upper_bound = if upper_bound == i64::MAX {
            None
        } else {
            Some(upper_bound)
        };

        Self {
            lower_bound,
            upper_bound,
        }
    }
}

pub struct GuardConstraints {
    pub variables: HashMap<usize, VariableBounds>,
}

impl GuardConstraints {
    pub fn apply_to_variable_ranges(
        &self,
        variable_bounds: &crate::context::VariableRanges,
    ) -> crate::context::VariableRanges {
        let mut res = crate::context::VariableRanges {
            bounds: variable_bounds.bounds.clone(),
        };

        for (variable, bound) in &self.variables {
            if let Some(min) = bound.lower_bound {
                res.bounds[*variable].add_min_int_constraint(min);
            }
            if let Some(max) = bound.upper_bound {
                res.bounds[*variable].add_max_int_constraint(max);
            }
        }

        res
    }

    pub fn from_expression(expression: &Expression<VariableReference, Span>) -> Self {
        let mut constraints = VariablesConstraints::new();
        Self::collect_constraints(expression, &mut constraints);
        let mut variables = HashMap::new();
        for (variable, var_constraints) in constraints.variables {
            variables.insert(
                variable,
                VariableBounds::from_variable_constraints(&var_constraints),
            );
        }
        Self { variables }
    }

    fn collect_constraints(
        expression: &Expression<VariableReference, Span>,
        constraints: &mut VariablesConstraints,
    ) {
        use Constraint::*;
        match expression {
            Expression::Conjunction(left, right, _) => {
                Self::collect_constraints(left, constraints);
                Self::collect_constraints(right, constraints);
            }
            Expression::LessThan(lhs, rhs, _) => {
                if let Some((v, c)) = Self::construct_constraint(lhs, rhs, LessThan, GreaterThan) {
                    constraints.add_constraint(v, c);
                }
            }
            Expression::LessOrEqual(lhs, rhs, _) => {
                if let Some((v, c)) =
                    Self::construct_constraint(lhs, rhs, LessOrEqual, GreaterOrEqual)
                {
                    constraints.add_constraint(v, c);
                }
            }
            Expression::GreaterThan(lhs, rhs, _) => {
                if let Some((v, c)) = Self::construct_constraint(lhs, rhs, GreaterThan, LessThan) {
                    constraints.add_constraint(v, c);
                }
            }
            Expression::GreaterOrEqual(lhs, rhs, _) => {
                if let Some((v, c)) =
                    Self::construct_constraint(lhs, rhs, GreaterOrEqual, LessOrEqual)
                {
                    constraints.add_constraint(v, c);
                }
            }
            Expression::Equals(lhs, rhs, _) => {
                if let Some((v, c)) = Self::construct_constraint(lhs, rhs, Equals, Equals) {
                    constraints.add_constraint(v, c);
                }
            }
            _ => (),
        }
    }

    fn construct_constraint<F1: Fn(i64) -> Constraint, F2: Fn(i64) -> Constraint>(
        lhs: &Box<Expression<VariableReference, Span>>,
        rhs: &Box<Expression<VariableReference, Span>>,
        constructor: F1,
        reverse_constructor: F2,
    ) -> Option<(VariableReference, Constraint)> {
        if let Expression::VarOrConst(var, _) = &**lhs {
            if let Expression::Int(val, _) = &**rhs {
                return Some((var.clone(), constructor(*val)));
            }
        }
        if let Expression::VarOrConst(var, _) = &**rhs {
            if let Expression::Int(val, _) = &**lhs {
                return Some((var.clone(), reverse_constructor(*val)));
            }
        }

        None
    }
}
