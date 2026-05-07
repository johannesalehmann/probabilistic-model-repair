use crate::prism::FeasibleCombination;
use crate::prism_output::{OutputVariableValues, VariableReferenceToPrismIndex};
use crate::repairs::RepairCollection;
use prism_model::{Expression, VariableManager, VariableReference};
use prism_parser::Span;

pub struct Fix {
    pub location: Span,
    pub replace_by: String,
    pub fix_type: FixType,
}

impl Fix {
    pub fn new(location: Span, replace_by: String, fix_type: FixType) -> Self {
        Self {
            location,
            replace_by,
            fix_type,
        }
    }
}

pub enum FixType {
    Fixed,
    NoChange,
}
pub struct AppliedRepair {
    costs: Vec<f64>,
    pub total_cost: f64,
    pub fixes: Vec<Fix>,
}

impl AppliedRepair {
    pub fn from_feasible_combination(
        feasible_combination: &FeasibleCombination,
        ref_to_prism: &VariableReferenceToPrismIndex,
        repair_collection: &RepairCollection,
        variable_manager: &VariableManager<Expression<VariableReference, Span>, Span>,
    ) -> Self {
        let mut res = AppliedRepair {
            costs: Vec::new(),
            total_cost: 0.0,
            fixes: Vec::new(),
        };

        let values = OutputVariableValues::new(&feasible_combination.variables, ref_to_prism);

        for repair in &repair_collection.repairs {
            let cost = repair.get_cost_and_fixes(&mut res.fixes, &values, variable_manager);
            res.costs.push(cost);
            res.total_cost += cost;
        }

        res
    }
}

pub struct AppliedRepairCollection {
    pub applied_repairs: Vec<AppliedRepair>,
}

impl AppliedRepairCollection {
    pub fn from_feasible_combinations(
        feasible_combinations: Vec<FeasibleCombination>,
        ref_to_prism: &VariableReferenceToPrismIndex,
        repair_collection: &RepairCollection,
        variable_manager: &VariableManager<Expression<VariableReference, Span>, Span>,
    ) -> Self {
        Self {
            applied_repairs: feasible_combinations
                .iter()
                .map(|f| {
                    AppliedRepair::from_feasible_combination(
                        f,
                        ref_to_prism,
                        repair_collection,
                        variable_manager,
                    )
                })
                .collect(),
        }
    }

    pub fn sort(&mut self) {
        self.applied_repairs
            .sort_unstable_by(|r1, r2| r1.total_cost.partial_cmp(&r2.total_cost).unwrap())
    }
}
