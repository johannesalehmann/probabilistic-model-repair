use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{DependencyOutputs, Modifications, Task};
use prism_model::{Displayable, Expression, VariableReference, VariableScope};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub struct SetupTask {}

impl SetupTask {
    pub fn new() -> Self {
        Self {}
    }
}

impl Task for SetupTask {
    fn description(&self) -> String {
        "SynthesisSetupTask".to_string()
    }

    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: DependencyOutputs,
        modifications: &mut Modifications,
    ) -> Box<dyn Any> {
        for (index, module) in model.modules.iter().enumerate() {
            if module.attributes.is_flag_set("repairable") {
                let task = Box::new(RepairTask {
                    module: RepairModule::ExistingModule { index },
                });
                modifications.create_task(task, vec![own_index]);
            }
        }

        modifications.create_task(
            Box::new(RepairTask {
                module: RepairModule::NewModule,
            }),
            vec![own_index],
        );

        Box::new(())
    }
}

#[derive(Copy, Clone)]
enum RepairModule {
    NewModule,
    ExistingModule { index: usize },
}

impl Display for RepairModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RepairModule::NewModule => {
                write!(f, "new module")
            }
            RepairModule::ExistingModule { index } => {
                write!(f, "replace module {index}")
            }
        }
    }
}

pub struct RepairTask {
    module: RepairModule,
}

impl Task for RepairTask {
    fn description(&self) -> String {
        format!("SynthesisTask({})", self.module)
    }

    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: DependencyOutputs,
        modifications: &mut Modifications,
    ) -> Box<dyn Any> {
        let interface = ModuleInterface::from_model(model, self.module);

        println!("Module interface:");
        println!("  Input variables:");
        for var in &interface.inputs.visible_variables {
            println!("    {}", model.variable_manager.get(var).unwrap().name);
        }
        println!("  Output variables:");
        for var in &interface.outputs.manipulatable_variables {
            println!("    {}", model.variable_manager.get(var).unwrap().name);
        }
        println!("  Actions:");
        for action in &interface.outputs.actions {
            println!(
                "    {} with guard {}",
                action.name,
                action.guard.displayable(&model.variable_manager)
            );
        }

        Box::new(())
    }
}

struct ModuleInterface {
    inputs: ModuleInputs,
    outputs: ModuleOutputs,
}

impl ModuleInterface {
    fn from_model(model: &PrismModel, module: RepairModule) -> Self {
        let mut visible_variables = Vec::new();
        let mut manipulatable_variables = Vec::new();
        for (index, variable) in model.variable_manager.variables.iter().enumerate() {
            if variable.is_constant() {
                continue;
            }
            let in_own_module = if let RepairModule::ExistingModule { index } = module {
                variable.scope
                    != (VariableScope::LocalVariable {
                        module_index: index,
                    })
            } else {
                false
            };
            if in_own_module {
                manipulatable_variables.push(VariableReference::new(index));
            } else {
                if variable.attributes.is_flag_set("hidden") {
                    continue;
                }
                visible_variables.push(VariableReference::new(index));
            }
        }
        let inputs = ModuleInputs { visible_variables };

        let mut actions_and_guards: HashMap<String, Expression> = HashMap::new();
        for module in &model.modules {
            let mut module_actions: HashMap<String, Expression> = HashMap::new();
            for command in module.commands.iter() {
                if let Some(action) = &command.action {
                    let guard = if let Some(guard) = module_actions.remove(&action.name) {
                        guard.or(command.guard.clone())
                    } else {
                        command.guard.clone()
                    };
                    module_actions.insert(action.name.clone(), guard);
                }
            }
            for (name, guard) in module_actions {
                let guard = if let Some(existing_guard) = actions_and_guards.remove(&name) {
                    existing_guard.and(guard)
                } else {
                    guard
                };
                actions_and_guards.insert(name, guard);
            }
        }

        let actions = actions_and_guards
            .into_iter()
            .map(|(name, guard)| ActionAndGuard { name, guard })
            .collect();

        let outputs = ModuleOutputs {
            manipulatable_variables,
            actions,
        };

        Self { inputs, outputs }
    }
}

struct ModuleInputs {
    visible_variables: Vec<VariableReference>,
}

struct ModuleOutputs {
    manipulatable_variables: Vec<VariableReference>,
    actions: Vec<ActionAndGuard>,
}

struct ActionAndGuard {
    name: String,
    guard: Expression,
}
