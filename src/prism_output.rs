use prism_model::{Expression, Identifier, Model, VariableReference};
use prism_parser::Span;
use std::collections::HashMap;

pub struct OutputVariableValues<'a, 'b> {
    values: &'a Vec<String>,
    ref_to_prism: &'b VariableReferenceToPrismIndex,
}

impl<'a, 'b> OutputVariableValues<'a, 'b> {
    pub fn new(values: &'a Vec<String>, ref_to_prism: &'b VariableReferenceToPrismIndex) -> Self {
        Self {
            values,
            ref_to_prism,
        }
    }

    pub fn get_value(&self, reference: VariableReference) -> &str {
        &self.values[self.ref_to_prism.ref_to_index(&reference)]
    }

    pub fn get_int(&self, reference: VariableReference) -> i64 {
        self.get_value(reference).parse().unwrap()
    }

    pub fn get_bool(&self, reference: VariableReference) -> bool {
        match self.get_value(reference) {
            "true" => true,
            "false" => false,
            val => panic!("Cannot turn `{val}` into bool"),
        }
    }
}

pub struct VariableReferenceToPrismIndex {
    map: HashMap<usize, usize>,
}

impl VariableReferenceToPrismIndex {
    pub fn from_model(
        model: &Model<
            (),
            Identifier<Span>,
            Expression<VariableReference, Span>,
            VariableReference,
            Span,
        >,
    ) -> Self {
        let mut map = HashMap::new();

        let mut index = 0;
        for (variable_index, variable) in model.variable_manager.variables.iter().enumerate() {
            if variable.scope.is_none() && !variable.is_constant {
                map.insert(variable_index, index);
                index += 1;
            }
        }
        for module_index in 0..model.modules.modules.len() {
            for (variable_index, variable) in model.variable_manager.variables.iter().enumerate() {
                if variable.scope == Some(module_index) && !variable.is_constant {
                    map.insert(variable_index, index);
                    index += 1;
                }
            }
        }

        Self { map }
    }

    pub fn ref_to_index(&self, reference: &VariableReference) -> usize {
        self.map[&reference.index]
    }
}
