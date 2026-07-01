use crate::repair_graph::{PrismModel, PropertyCollection};
use crate::task_graph::{Modifications, OutputsOfDependencies, TaskDescription};
use prism_model::{
    Assignment, Command, Displayable, Expression, FullSpan, Identifier, Module, Update,
    VariableInfo, VariableManager, VariableRange, VariableReference, VariableScope,
};
use probabilistic_properties::{BoundOperator, NonDeterminismKind, StateFormula};
use std::any::Any;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::process::Stdio;

pub struct SetupTask {}

impl SetupTask {
    pub fn new() -> Self {
        Self {}
    }
}

impl TaskDescription for SetupTask {
    fn name(&self) -> String {
        "SynthesisSetupTask".to_string()
    }

    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: OutputsOfDependencies,
        modifications: &mut Modifications,
        temp_directory: &Path,
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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

impl TaskDescription for RepairTask {
    fn name(&self) -> String {
        format!("SynthesisTask({})", self.module)
    }

    fn run(
        &mut self,
        model: &PrismModel,
        properties: &PropertyCollection,
        own_index: usize,
        dependency_outputs: OutputsOfDependencies,
        modifications: &mut Modifications,
        temp_directory: &Path,
    ) -> Box<dyn Any> {
        let interface = ModuleInterface::from_model(model, self.module);
        // interface.print(&model.variable_manager);

        let mut game = model.clone();

        let repair_module_index = match self.module {
            RepairModule::NewModule => {
                let index = game.modules.len();
                game.modules
                    .add(Module::new(
                        Identifier::new("generated_repair_module").unwrap(),
                    ))
                    .unwrap();
                index
            }
            RepairModule::ExistingModule { index } => {
                game.modules.get_mut(index).unwrap().commands.clear();
                index
            }
        };

        let choice_var = game
            .variable_manager
            .add_variable(VariableInfo::global_var(
                Identifier::new("action_choice").unwrap(),
                VariableRange::bounded_int(
                    Expression::int(0),
                    Expression::int(interface.outputs.actions.len() as i64),
                ),
            ))
            .unwrap();
        let phase_var = game
            .variable_manager
            .add_variable(VariableInfo::global_var(
                Identifier::new("phase").unwrap(),
                VariableRange::bounded_int(Expression::int(0), Expression::int(1)),
            ))
            .unwrap();

        let mut variable_domains = Vec::new();
        for variable in &interface.outputs.manipulatable_variables {
            let mut values = Vec::new();
            match &model.variable_manager.get(variable).unwrap().range {
                VariableRange::BoundedInt { min, max, .. } => {
                    let (min, max) = match (min, max) {
                        (Expression::Int(min, _), Expression::Int(max, _)) => (*min, *max),
                        _ => panic!(
                            "Module synthesis can only handle bounded integers whose bounds are given as integer literals"
                        ),
                    };
                    for i in min..=max {
                        values.push(Expression::int(i));
                    }
                }
                VariableRange::UnboundedInt { .. } => {
                    panic!(
                        "Cannot synthesise module when unbounded integer is accessible to repair module"
                    )
                }
                VariableRange::Boolean { .. } => {
                    values.push(Expression::bool(false));
                    values.push(Expression::bool(true));
                }
                VariableRange::Float { .. } => {
                    panic!("Cannot synthesise module when double is accessible to repair module")
                }
            }
            variable_domains.push(values);
        }

        let mut repair_controlled = Vec::new();

        let mut choose_actions_info = HashMap::new();
        // Add commands to new module
        for (action_index, action) in interface.outputs.actions.iter().enumerate() {
            let mut domain_indices = vec![0; interface.outputs.manipulatable_variables.len()];
            loop {
                let mut name_components = vec!["Choose_".to_string()];
                name_components.push(action.name.clone());
                for (&index, domain) in domain_indices.iter().zip(variable_domains.iter()) {
                    name_components.push("_".to_string());
                    name_components.push(
                        domain[index]
                            .displayable(&model.variable_manager)
                            .to_string(),
                    );
                }
                let name = name_components.join("");
                repair_controlled.push(format!("[{name}]"));

                let mut required_assignments = Vec::new();

                let mut command = Command::new(
                    Some(Identifier::new(name.to_string()).unwrap()),
                    Expression::var_or_const(phase_var)
                        .equals_to(Expression::int(0))
                        .and(action.guard.clone()),
                );
                let mut update = Update::new(Expression::float(1.0));
                update.add_assignment(Assignment::new(phase_var, Expression::int(1)));
                update.add_assignment(Assignment::new(
                    choice_var,
                    Expression::int(action_index as i64),
                ));

                for ((&index, domain), &variable) in domain_indices
                    .iter()
                    .zip(variable_domains.iter())
                    .zip(interface.outputs.manipulatable_variables.iter())
                {
                    required_assignments.push((variable, domain[index].clone()));
                    update
                        .assignments
                        .push(Assignment::new(variable, domain[index].clone()));
                }
                command.updates.push(update);

                game.modules
                    .get_mut(repair_module_index)
                    .unwrap()
                    .commands
                    .push(command);

                choose_actions_info
                    .insert(name.clone(), (action.name.clone(), required_assignments));

                if domain_indices.len() == 0 {
                    break;
                }
                // Increment domain index:
                domain_indices[0] += 1;
                let mut index = 0;
                let mut done = false;
                while domain_indices[index] >= variable_domains[index].len() {
                    domain_indices[index] = 0;
                    index += 1;
                    if index >= domain_indices.len() {
                        done = true;
                        break;
                    } else {
                        domain_indices[index] += 1;
                    }
                }
                if done {
                    break;
                }
            }
        }

        let mut environment_controlled = Vec::new();

        // Add restrictions to existing commands:
        for (module_index, module) in game.modules.iter_mut().enumerate() {
            if module_index == repair_module_index {
                continue;
            }
            environment_controlled.push(module.name.name.clone());
            for command in &mut module.commands {
                if let Some(action) = &command.action {
                    let in_parentheses = format!("[{}]", action.name);
                    if !environment_controlled.contains(&in_parentheses) {
                        environment_controlled.push(in_parentheses);
                    }
                    let action_index = interface
                        .outputs
                        .actions
                        .iter()
                        .position(|a| a.name == action.name)
                        .unwrap();

                    let phase_check =
                        Expression::var_or_const(phase_var.clone()).equals_to(Expression::int(1));
                    let action_check = Expression::var_or_const(choice_var.clone())
                        .equals_to(Expression::int(action_index as i64));

                    let temp = std::mem::replace(&mut command.guard, Expression::bool(true));
                    command.guard = phase_check.and(action_check).and(temp);

                    for update in &mut command.updates {
                        update
                            .assignments
                            .push(Assignment::new(phase_var, Expression::int(0)))
                    }
                }
            }
        }

        let mut game_source = game.to_string();
        game_source = game_source.replacen("mdp", "smg", 1);

        game_source = format!(
            "{game_source}\n\nplayer Repair\n    {}\nendplayer\n\nplayer Environment\n    {}\nendplayer",
            repair_controlled.join(", "),
            environment_controlled.join(", ")
        );

        let game_file_name = temp_directory.join("synthesis_game.prism");
        let game_props_name = temp_directory.join("synthesis_game.props");

        std::fs::write(&game_file_name, game_source).unwrap();

        let mut game_properties = Vec::new();
        for property in &properties.properties {
            let mut property = property.clone();
            if let probabilistic_properties::Query::StateFormula(StateFormula::ProbabilityBound {
                non_determinism,
                bound,
                path,
            }) = &mut property
            {
                match (&non_determinism, bound.operator) {
                    (Some(NonDeterminismKind::Minimise), BoundOperator::GreaterThan)
                    | (Some(NonDeterminismKind::Minimise), BoundOperator::GreaterOrEqual) => {
                        *non_determinism = None;
                    }
                    (Some(NonDeterminismKind::Maximise), BoundOperator::LessThan)
                    | (Some(NonDeterminismKind::Maximise), BoundOperator::LessOrEqual) => {
                        *non_determinism = None;
                    }
                    (Some(NonDeterminismKind::Minimise), BoundOperator::LessThan) => {
                        panic!("PRISM cannot check a property of form `Pmin < t`")
                    }
                    (Some(NonDeterminismKind::Minimise), BoundOperator::LessOrEqual) => {
                        panic!("PRISM cannot check a property of form `Pmin <= t`")
                    }
                    (Some(NonDeterminismKind::Maximise), BoundOperator::GreaterThan) => {
                        panic!("PRISM cannot check a property of form `Pmax > t`")
                    }
                    (Some(NonDeterminismKind::Maximise), BoundOperator::GreaterOrEqual) => {
                        panic!("PRISM cannot check a property of form `Pmax > t`")
                    }

                    _ => (),
                }
            }
            game_properties.push(format!(
                "<<Repair>> {}",
                property
                    .map_i(&mut |e| e.displayable(&model.variable_manager).to_string())
                    .map_f(&mut |e| e.displayable(&model.variable_manager).to_string())
                    .map_e(&mut |e| e.displayable(&model.variable_manager).to_string())
            ));
        }
        std::fs::write(&game_props_name, game_properties.join(";\n")).unwrap();

        let strategy_file_name = temp_directory.join("synthesis_game.strat");
        let states_file = temp_directory.join("synthesis_game.sta");

        let strategy_option = format!("{}:type=induced", strategy_file_name.to_str().unwrap());
        let process = match std::process::Command::new("prism-games")
            .args(&[
                game_file_name.as_os_str(),
                game_props_name.as_os_str(),
                OsStr::new("-exportstrat"),
                OsStr::new(&strategy_option),
                OsStr::new("-exportmodel"),
                states_file.as_os_str(),
            ])
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(process) => process,
            Err(err) => panic!("Error spawning `prism-games`: {}", err),
        };

        let output = match process.wait_with_output() {
            Ok(output) => output,
            Err(err) => panic!("Could not read `prism-games` output: {}", err),
        };

        let stdout = match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(err) => panic!("`prism-games` output is not valid utf8: {}", err),
        };
        let was_true = stdout.contains("Result: true");
        let was_false = stdout.contains("Result: false");
        if (was_false && was_true) || (!was_false && !was_true) {
            panic!(
                "Prism games returned inconclusive output (was_true = {was_true} and was_false = {was_false}"
            );
        }

        if was_true {
            let states = StateFile::from_sta(&std::fs::read_to_string(states_file).unwrap());
            let induced_game =
                InducedGame::from_strat(&std::fs::read_to_string(strategy_file_name).unwrap());

            let mut visible_variable_combinations = VisibleVariableCollector::new(
                &interface.inputs.visible_variables,
                &model.variable_manager,
            );

            for action in induced_game.entries {
                let state = &states.states[action.from];
                if state.get_int("phase").unwrap() == 0 {
                    let entry = visible_variable_combinations.get_mut(state).unwrap();
                    entry.add_action(&action.action_name.unwrap());
                }
            }

            let mut final_model = model.clone();
            let module_index = match self.module {
                RepairModule::NewModule => {
                    let res = final_model.modules.len();
                    final_model
                        .modules
                        .add(Module::new(Identifier::new("synthesised_module").unwrap()))
                        .unwrap();
                    res
                }
                RepairModule::ExistingModule { index } => {
                    final_model.modules.get_mut(index).unwrap().commands.clear();
                    index
                }
            };

            let mut issues = false;
            for (name, actions) in visible_variable_combinations.value_to_actions {
                if actions.required_actions.len() > 1 {
                    issues = true;
                    println!(
                        "Multiple actions required for same visible variables valuation ({name}):"
                    );
                    for action in &actions.required_actions {
                        println!("    {action}")
                    }
                }
                if actions.required_actions.len() == 0 {
                    continue;
                }
                let choose_action = &actions.required_actions[0];

                let (action, assignments) =
                    choose_actions_info.get(choose_action).unwrap_or_else(|| {
                        panic!("Cannot find choose action for name `{}`", choose_action)
                    });

                let module = final_model.modules.get_mut(module_index).unwrap();
                let mut command =
                    Command::new(Some(Identifier::new(action).unwrap()), actions.condition);
                let mut update: Update = Update::new(Expression::int(1));
                for (target, value) in assignments {
                    update
                        .assignments
                        .push(Assignment::new(target.clone(), value.clone()));
                }
                command.updates.push(update);
                module.commands.push(command);
            }
            println!("  Synthesised a controller module.");
            modifications.create_repair_graph_node(final_model, properties.clone());
        } else {
            println!("  Could not synthesise a suitable controller.")
        }

        Box::new(())
    }
}

impl RepairTask {
    fn domain_at_end(domain_indices: &Vec<usize>, domains: &Vec<Vec<Expression>>) -> bool {
        domain_indices
            .iter()
            .zip(domains.iter())
            .all(|(index, domain)| *index < domain.len())
    }
}

struct ModuleInterface {
    inputs: ModuleInputs,
    outputs: ModuleOutputs,
}

impl ModuleInterface {
    fn from_model(model: &PrismModel, module_to_repair: RepairModule) -> Self {
        let mut visible_variables = Vec::new();
        let mut manipulatable_variables = Vec::new();
        for (index, variable) in model.variable_manager.variables.iter().enumerate() {
            if variable.is_constant() {
                continue;
            }
            let in_own_module = if let RepairModule::ExistingModule { index } = module_to_repair {
                variable.scope
                    == (VariableScope::LocalVariable {
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
        for (module_index, module) in model.modules.iter().enumerate() {
            if module_to_repair
                == (RepairModule::ExistingModule {
                    index: module_index,
                })
            {
                // We will throw away the actions of this module, so it does not make sense to keep
                // their guards in the restrictions.
                continue;
            }
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

    fn print(&self, variable_manager: &VariableManager) {
        println!("Module interface:");
        println!("  Input variables:");
        for var in &self.inputs.visible_variables {
            println!("    {}", variable_manager.get(var).unwrap().name);
        }
        println!("  Output variables:");
        for var in &self.outputs.manipulatable_variables {
            println!("    {}", variable_manager.get(var).unwrap().name);
        }
        println!("  Actions:");
        for action in &self.outputs.actions {
            println!(
                "    {} with guard {}",
                action.name,
                action.guard.displayable(&variable_manager)
            );
        }
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

struct InducedGame {
    entries: Vec<InducedGameEntry>,
}

impl InducedGame {
    pub fn from_strat(source: &str) -> Self {
        let mut entries = Vec::new();

        let mut lines = source.lines();
        let _first = lines.next();
        for line in lines {
            let components = line.split(" ").collect::<Vec<_>>();
            let from = components[0].parse().unwrap();
            let to = components[1].parse().unwrap();
            let probability = components[2].parse().unwrap();
            let action_name = components.get(3).map(|a| a.to_string());

            entries.push(InducedGameEntry {
                from,
                to,
                probability,
                action_name,
            })
        }

        Self { entries }
    }
}

struct InducedGameEntry {
    from: usize,
    to: usize,
    probability: f64,
    action_name: Option<String>,
}

struct StateFile {
    states: Vec<StateFileEntry>,
}

impl StateFile {
    fn from_sta(source: &str) -> Self {
        let mut states = Vec::new();

        let mut lines = source.lines();
        let first = lines.next().unwrap().trim();
        let first = &first[1..first.len() - 1];
        let variable_names = first.split(",").collect::<Vec<_>>();
        for line in lines {
            let (index, values) = line.split_once(":").unwrap();
            assert_eq!(index, states.len().to_string());
            let values = &values[1..values.len() - 1];

            let mut variables = HashMap::new();
            for (variable, value) in variable_names.iter().zip(values.split(",")) {
                let ex = match value {
                    "true" => Expression::bool(true),
                    "false" => Expression::bool(false),
                    val => Expression::int(val.parse().unwrap()),
                };
                variables.insert(variable.to_string(), ex);
            }
            states.push(StateFileEntry { variables })
        }
        Self { states }
    }
}

struct StateFileEntry {
    variables: HashMap<String, Expression>,
}

impl StateFileEntry {
    fn get_int(&self, name: &str) -> Option<i64> {
        let ex = self.variables.get(name)?;
        if let Expression::Int(val, _) = ex {
            Some(*val)
        } else {
            None
        }
    }
    fn get_bool(&self, name: &str) -> Option<bool> {
        let ex = self.variables.get(name)?;
        if let Expression::Bool(val, _) = ex {
            Some(*val)
        } else {
            None
        }
    }
}

struct VisibleVariableCollector {
    value_to_actions: HashMap<String, VisibleVariableCollectorEntry>,
    variable_names: Vec<String>,
}

impl VisibleVariableCollector {
    pub fn new(
        visible_variables: &Vec<VariableReference>,
        variable_manager: &VariableManager,
    ) -> Self {
        let mut value_to_actions = HashMap::new();

        let mut variable_domains = Vec::new();
        let mut variable_names = Vec::new();
        for variable in visible_variables {
            variable_names.push(variable_manager.get(variable).unwrap().name.name.clone());
            let mut values = Vec::new();
            match &variable_manager.get(variable).unwrap().range {
                VariableRange::BoundedInt { min, max, .. } => {
                    let (min, max) = match (min, max) {
                        (Expression::Int(min, _), Expression::Int(max, _)) => (*min, *max),
                        _ => panic!(
                            "Module synthesis can only handle bounded integers whose bounds are given as integer literals"
                        ),
                    };
                    for i in min..=max {
                        values.push(Expression::int(i));
                    }
                }
                VariableRange::UnboundedInt { .. } => {
                    panic!(
                        "Cannot synthesise module when unbounded integer is accessible to repair module"
                    )
                }
                VariableRange::Boolean { .. } => {
                    values.push(Expression::bool(false));
                    values.push(Expression::bool(true));
                }
                VariableRange::Float { .. } => {
                    panic!("Cannot synthesise module when double is accessible to repair module")
                }
            }
            variable_domains.push(values);
        }

        let mut domain_indices = vec![0; variable_domains.len()];

        loop {
            let mut name_components = Vec::new();
            let mut condition: Option<Expression> = None;
            for (&reference, (&index, domain)) in visible_variables
                .iter()
                .zip(domain_indices.iter().zip(variable_domains.iter()))
            {
                let value: &Expression = &domain[index];
                name_components.push(value.displayable(variable_manager).to_string());
                let check = Expression::var_or_const(reference).equals_to(value.clone());

                let temp = std::mem::replace(&mut condition, None);
                if let Some(existing_condition) = temp {
                    condition = Some(existing_condition.and(check));
                } else {
                    condition = Some(check);
                }
            }
            let condition = condition.unwrap_or(Expression::bool(true));

            let name = name_components.join(",");

            value_to_actions.insert(name, VisibleVariableCollectorEntry::new(condition));

            if domain_indices.len() == 0 {
                break;
            }
            // Increment domain index:
            domain_indices[0] += 1;
            let mut index = 0;
            let mut done = false;
            while domain_indices[index] >= variable_domains[index].len() {
                domain_indices[index] = 0;
                index += 1;
                if index >= domain_indices.len() {
                    done = true;
                    break;
                } else {
                    domain_indices[index] += 1;
                }
            }
            if done {
                break;
            }
        }

        Self {
            value_to_actions,
            variable_names,
        }
    }

    pub fn get_mut(
        &mut self,
        state: &StateFileEntry,
    ) -> Option<&mut VisibleVariableCollectorEntry> {
        let mut name_components = Vec::new();
        for variable in &self.variable_names {
            if let Some(int) = state.get_int(variable) {
                name_components.push(int.to_string());
            } else if let Some(bool) = state.get_bool(variable) {
                name_components.push(bool.to_string());
            }
        }
        let name = name_components.join(",");

        self.value_to_actions.get_mut(&name)
    }
}

struct VisibleVariableCollectorEntry {
    condition: Expression,
    required_actions: Vec<String>,
}
impl VisibleVariableCollectorEntry {
    pub fn new(condition: Expression) -> Self {
        Self {
            condition,
            required_actions: Vec::new(),
        }
    }

    pub fn add_action(&mut self, action: &String) {
        if !self.required_actions.contains(action) {
            self.required_actions.push(action.clone())
        }
    }
}
