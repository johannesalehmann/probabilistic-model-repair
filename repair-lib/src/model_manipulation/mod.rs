use prism_model::{Expression, FullSpan, Model, Span, VariableRange, VariableReference};

pub struct SubExpressionOfModel {
    expression: ExpressionLocation,
    sub_expression: SubExpression,
}

pub enum ExpressionLocation {
    Formula {
        index: usize,
    },
    Label {
        index: usize,
    },
    VariableDeclaration {
        reference: VariableReference,
        component: VariableComponent,
    },
    Command {
        module_index: usize,
        command_index: usize,
        component: CommandComponent,
    },
    // TODO: Support renamed modules, init constraints, rewards, etc.
}

pub trait LocatableExpression {
    fn get_expression(&self, location: &ExpressionLocation) -> Option<&Expression>;
}

impl LocatableExpression for Model {
    fn get_expression(&self, location: &ExpressionLocation) -> Option<&Expression> {
        {
            match location {
                ExpressionLocation::Formula { index } => {
                    self.formulas.get(*index).map(|f| &f.condition)
                }
                ExpressionLocation::Label { index } => {
                    self.labels.get(*index).map(|l| &l.condition)
                }
                ExpressionLocation::VariableDeclaration {
                    reference,
                    component,
                } => {
                    let variable = self.variable_manager.get(reference);
                    match component {
                        VariableComponent::InitialValue => {
                            variable.and_then(|v| v.initial_value.as_ref())
                        }
                    }
                }
                ExpressionLocation::Command {
                    module_index,
                    command_index,
                    component,
                } => {
                    let command = self
                        .modules
                        .get(*module_index)
                        .and_then(|m| m.commands.get(*command_index));
                    match component {
                        CommandComponent::Guard => command.map(|c| &c.guard),
                        CommandComponent::Update { index, component } => {
                            let update = command.and_then(|c| c.updates.get(*index));
                            match component {
                                UpdateComponent::Probability => update.map(|u| &u.probability),
                                UpdateComponent::Assignment { index } => {
                                    update.and_then(|u| u.assignments.get(*index).map(|a| &a.value))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub enum VariableComponent {
    InitialValue,
}

pub enum CommandComponent {
    Guard,
    Update {
        index: usize,
        component: UpdateComponent,
    },
}

pub enum UpdateComponent {
    Probability,
    Assignment { index: usize },
}

#[derive(Clone)]
pub enum SubExpression {
    Here,
    Child {
        index: usize,
        child_navigation: Box<SubExpression>,
    },
}

impl SubExpression {
    fn child_then_here(index: usize) -> Self {
        SubExpression::Child {
            index,
            child_navigation: Box::new(SubExpression::Here),
        }
    }

    pub fn and_then(&self, then: SubExpression) -> SubExpression {
        match self {
            SubExpression::Here => then,
            SubExpression::Child {
                index,
                child_navigation,
            } => SubExpression::Child {
                index: index.clone(),
                child_navigation: Box::new(child_navigation.and_then(then)),
            },
        }
    }
}

pub trait NavigableExpression<V, S: Span> {
    fn explore<F: FnMut(&Expression<V, S>, SubExpression)>(&self, visit: F);
    fn explore_internal<F: FnMut(&Expression<V, S>, SubExpression)>(
        &self,
        visit: &mut F,
        navigation_to_self: SubExpression,
    );
    fn get(&self, sub_expression: &SubExpression) -> Option<&Expression<V, S>>;
    fn replace(&mut self, sub_expression: &SubExpression, replacement: Expression<V, S>);
}

impl<V, S: Span> NavigableExpression<V, S> for Expression<V, S> {
    fn explore<F: FnMut(&Expression<V, S>, SubExpression)>(&self, mut visit: F) {
        self.explore_internal(&mut visit, SubExpression::Here)
    }

    fn explore_internal<F: FnMut(&Expression<V, S>, SubExpression)>(
        &self,
        visit: &mut F,
        navigation_to_self: SubExpression,
    ) {
        visit(self, navigation_to_self.clone());
        match self {
            Expression::Int(_, _)
            | Expression::Float(_, _)
            | Expression::Bool(_, _)
            | Expression::VarOrConst(_, _)
            | Expression::Label(_, _) => (),
            Expression::Function(_, children, _) => {
                for (index, child) in children.iter().enumerate() {
                    child.explore_internal(
                        visit,
                        navigation_to_self.and_then(SubExpression::child_then_here(index)),
                    )
                }
            }
            Expression::Minus(inner, _) | Expression::Negation(inner, _) => {
                inner.explore_internal(visit, SubExpression::child_then_here(0))
            }
            Expression::Multiplication(lhs, rhs, _)
            | Expression::Division(lhs, rhs, _)
            | Expression::Addition(lhs, rhs, _)
            | Expression::Subtraction(lhs, rhs, _)
            | Expression::LessThan(lhs, rhs, _)
            | Expression::LessOrEqual(lhs, rhs, _)
            | Expression::GreaterThan(lhs, rhs, _)
            | Expression::GreaterOrEqual(lhs, rhs, _)
            | Expression::Equals(lhs, rhs, _)
            | Expression::NotEquals(lhs, rhs, _)
            | Expression::Conjunction(lhs, rhs, _)
            | Expression::Disjunction(lhs, rhs, _)
            | Expression::IfAndOnlyIf(lhs, rhs, _)
            | Expression::Implies(lhs, rhs, _) => {
                lhs.explore_internal(visit, SubExpression::child_then_here(0));
                rhs.explore_internal(visit, SubExpression::child_then_here(1));
            }
            Expression::Ternary(guard, lhs, rhs, _) => {
                guard.explore_internal(visit, SubExpression::child_then_here(0));
                lhs.explore_internal(visit, SubExpression::child_then_here(1));
                rhs.explore_internal(visit, SubExpression::child_then_here(2));
            }
        };
    }

    fn get(&self, sub_expression: &SubExpression) -> Option<&Expression<V, S>> {
        match sub_expression {
            SubExpression::Here => Some(self),
            SubExpression::Child {
                index,
                child_navigation,
            } => {
                let index = *index;
                let child =
                    match self {
                        Expression::Int(_, _)
                        | Expression::Float(_, _)
                        | Expression::Bool(_, _)
                        | Expression::VarOrConst(_, _)
                        | Expression::Label(_, _) => None,
                        Expression::Function(_, children, _) => children.get(index),
                        Expression::Minus(inner, _) | Expression::Negation(inner, _) => {
                            if index == 0 { Some(&**inner) } else { None }
                        }
                        Expression::Multiplication(lhs, rhs, _)
                        | Expression::Division(lhs, rhs, _)
                        | Expression::Addition(lhs, rhs, _)
                        | Expression::Subtraction(lhs, rhs, _)
                        | Expression::LessThan(lhs, rhs, _)
                        | Expression::LessOrEqual(lhs, rhs, _)
                        | Expression::GreaterThan(lhs, rhs, _)
                        | Expression::GreaterOrEqual(lhs, rhs, _)
                        | Expression::Equals(lhs, rhs, _)
                        | Expression::NotEquals(lhs, rhs, _)
                        | Expression::Conjunction(lhs, rhs, _)
                        | Expression::Disjunction(lhs, rhs, _)
                        | Expression::IfAndOnlyIf(lhs, rhs, _)
                        | Expression::Implies(lhs, rhs, _) => match index {
                            0 => Some(&**lhs),
                            1 => Some(&**rhs),
                            _ => None,
                        },
                        Expression::Ternary(guard, lhs, rhs, _) => match index {
                            0 => Some(&**guard),
                            1 => Some(&**lhs),
                            2 => Some(&**rhs),
                            _ => None,
                        },
                    };

                child?.get(child_navigation)
            }
        }
    }

    fn replace(&mut self, sub_expression: &SubExpression, replacement: Expression<V, S>) {
        let panic = |i, n| {
            panic!(
                "Cannot replace child {i} of expression with {n} {}.",
                if n == 1 { "child" } else { "children" }
            )
        };

        match sub_expression {
            SubExpression::Here => *self = replacement,
            SubExpression::Child {
                index,
                child_navigation,
            } => {
                let index = *index;
                let child = match self {
                    Expression::Int(_, _)
                    | Expression::Float(_, _)
                    | Expression::Bool(_, _)
                    | Expression::VarOrConst(_, _)
                    | Expression::Label(_, _) => panic(index, 0),
                    Expression::Function(_, children, _) => {
                        if index < children.len() {
                            children[index].replace(child_navigation, replacement)
                        } else {
                            panic(index, children.len());
                        }
                    }
                    Expression::Minus(inner, _) | Expression::Negation(inner, _) => {
                        if index == 0 {
                            inner.replace(child_navigation, replacement)
                        } else {
                            panic(index, 1)
                        }
                    }
                    Expression::Multiplication(lhs, rhs, _)
                    | Expression::Division(lhs, rhs, _)
                    | Expression::Addition(lhs, rhs, _)
                    | Expression::Subtraction(lhs, rhs, _)
                    | Expression::LessThan(lhs, rhs, _)
                    | Expression::LessOrEqual(lhs, rhs, _)
                    | Expression::GreaterThan(lhs, rhs, _)
                    | Expression::GreaterOrEqual(lhs, rhs, _)
                    | Expression::Equals(lhs, rhs, _)
                    | Expression::NotEquals(lhs, rhs, _)
                    | Expression::Conjunction(lhs, rhs, _)
                    | Expression::Disjunction(lhs, rhs, _)
                    | Expression::IfAndOnlyIf(lhs, rhs, _)
                    | Expression::Implies(lhs, rhs, _) => match index {
                        0 => lhs.replace(child_navigation, replacement),
                        1 => rhs.replace(child_navigation, replacement),
                        _ => panic(index, 2),
                    },
                    Expression::Ternary(guard, lhs, rhs, _) => match index {
                        0 => guard.replace(child_navigation, replacement),
                        1 => lhs.replace(child_navigation, replacement),
                        2 => rhs.replace(child_navigation, replacement),
                        _ => panic(index, 3),
                    },
                };
            }
        }
    }
}
