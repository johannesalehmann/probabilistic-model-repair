use crate::repair_graph::PrismModel;
use prism_model::{Attribute, Identifier, VariableScope};

pub fn preprocess_model(model: &mut PrismModel) {
    annotate_fixed(model);
    annotate_visible(model);
}

fn annotate_fixed(model: &mut PrismModel) {
    let (mut saw_fixed, mut saw_repairable, mut saw_unmarked) = (false, false, false);
    for module in &model.modules {
        let fixed = module.attributes.is_flag_set("fixed");
        let repairable = module.attributes.is_flag_set("repairable");
        if fixed && repairable {
            panic!(
                "Module `{}` is fixed and repairable at the same time",
                module.name
            );
        }
        saw_fixed = saw_fixed || fixed;
        saw_repairable = saw_repairable || repairable;
        saw_unmarked = saw_unmarked || (!fixed && !repairable);
    }
    if !saw_fixed && !saw_repairable && saw_unmarked {
        println!(
            "Modules are not annotated with `{{fixed}}` or `{{repairable}}`. Assuming that every module is repairable."
        )
    }
    if saw_fixed && saw_repairable && saw_unmarked {
        panic!(
            "Some modules are marked with `{{fixed}}` and `{{repairable}}`, while others are unmarked. Either mark every module or only mark either fixed or repairable modules. In the latter case, the other modules are implicitly assumed to be of opposite type"
        )
    };
    let unmarked_attribute = if saw_repairable {
        "fixed"
    } else {
        "repairable"
    };
    for module in &mut model.modules {
        let fixed = module.attributes.is_flag_set("fixed");
        let repairable = module.attributes.is_flag_set("repairable");
        if !fixed && !repairable {
            module
                .attributes
                .add(Attribute::flag(
                    Identifier::new(unmarked_attribute).unwrap(),
                ))
                .unwrap();
        }
    }
}

fn annotate_visible(model: &mut PrismModel) {
    for variable in &mut model.variable_manager.variables {
        if variable.attributes.get("visible").is_none()
            && variable.attributes.get("hidden").is_none()
        {
            if let VariableScope::LocalVariable { module_index } = variable.scope {
                let module = model.modules.get(module_index).unwrap();
                let visible = module.attributes.is_flag_set("visible");
                let hidden = module.attributes.is_flag_set("hidden");
                if visible && hidden {
                    panic!(
                        "Module `{}` is marked both as visible and hidden",
                        module.name
                    );
                }
                if visible {
                    variable
                        .attributes
                        .add(Attribute::flag(Identifier::new("visible").unwrap()))
                        .unwrap();
                }
                if hidden {
                    variable
                        .attributes
                        .add(Attribute::flag(Identifier::new("hidden").unwrap()))
                        .unwrap();
                }
            }
        }
    }

    let (mut saw_visible, mut saw_hidden, mut saw_unmarked) = (false, false, false);
    for variable in &model.variable_manager.variables {
        let visible = variable.attributes.is_flag_set("visible");
        let hidden = variable.attributes.is_flag_set("hidden");

        if let VariableScope::LocalVariable { module_index } = variable.scope {
            let module = model.modules.get(module_index).unwrap();
            if module.attributes.is_flag_set("repairable") {
                if visible || hidden {
                    panic!(
                        "Variables in repairable modules must not be marked as visible or hidden"
                    );
                }
                continue;
            }
        }

        if visible && hidden {
            panic!(
                "Variable `{}` is marked both as visible and hidden",
                variable.name
            )
        }

        saw_visible = saw_visible || visible;
        saw_hidden = saw_hidden || hidden;
        saw_unmarked = saw_unmarked || (!visible && !hidden);
    }

    if !saw_visible && !saw_hidden && saw_unmarked {
        println!(
            "Assuming all variables are visible, as no variables (or modules) were marked with `{{visible}}` or `{{hidden}}`."
        )
    }

    if saw_visible && saw_hidden && saw_unmarked {
        panic!(
            "Some variables are marked as visible or hidden, while other are left unmarked. Either mark every variable (except in repairable modules) or mark either only visible or only hidden variables."
        )
    }

    let unmarked_attribute = if saw_visible { "hidden" } else { "visible" };

    for variable in &mut model.variable_manager.variables {
        if let VariableScope::LocalVariable { module_index } = variable.scope {
            let module = model.modules.get(module_index).unwrap();
            if module.attributes.is_flag_set("repairable") {
                continue;
            }
        }
        let visible = variable.attributes.is_flag_set("visible");
        let hidden = variable.attributes.is_flag_set("hidden");
        if !visible && !hidden {
            variable
                .attributes
                .add(Attribute::flag(
                    Identifier::new(unmarked_attribute).unwrap(),
                ))
                .unwrap();
        }
    }
}
