use crate::ui::repair_graph::window_builder::WindowState;
use iced::Vector;
use rand::RngExt;
use repair_lib::repair_graph::RepairGraph;

pub struct LayoutOptions {
    pub node_width: f32,
    pub node_height: f32,
    pub vertical_spacing: f32,
    pub width: f32,
}

impl LayoutOptions {
    pub fn new() -> Self {
        Self {
            node_width: 300.0,
            node_height: 50.0,
            vertical_spacing: 80.0,
            width: 500.0,
        }
    }
}

pub struct RepairGraphLayout {
    pub layout: Vec<ModelLayout>,
    pub options: LayoutOptions,
}

impl RepairGraphLayout {
    pub fn new() -> Self {
        Self {
            layout: Vec::new(),
            options: LayoutOptions::new(),
        }
    }

    pub fn update_for_graph(&mut self, graph: &RepairGraph) {
        let mut changed = false;
        for (model_index, model_node) in graph.nodes.iter().enumerate() {
            if self.layout.len() <= model_index {
                let initial_position = model_node
                    .parent
                    .as_ref()
                    .and_then(|p| self.task_position(p.model_index, p.task_index))
                    .map(|p| p.position + Vector::new(0.0, 150.0))
                    .unwrap_or(Vector::new(0.0, 0.0));

                self.layout.push(ModelLayout {
                    model_position: LayoutNode::new(initial_position, 0),
                    task_positions: Vec::new(),
                });
                changed = true;
            }
            for (task_index, task_node) in model_node.tasks.tasks.iter().enumerate() {
                if self.layout[model_index].task_positions.len() <= task_index {
                    let initial_position = task_node
                        .dependencies
                        .get(0)
                        .and_then(|i| self.task_position(model_index, *i))
                        .or(self.model_position(model_index))
                        .map(|p| p.position + Vector::new(0.0, 150.0))
                        .unwrap_or(Vector::ZERO);
                    self.layout[model_index]
                        .task_positions
                        .push(LayoutNode::new(initial_position, 8));
                    changed = true;
                }
            }
        }
    }

    fn anneal(&mut self, iterations: usize, graph: &RepairGraph) {
        let mut nodes: Vec<(usize, Option<usize>)> = Vec::new();
        for (model_index, model) in self.layout.iter().enumerate() {
            nodes.push((model_index, None));
            for (task_index, task) in model.task_positions.iter().enumerate() {
                nodes.push((model_index, Some(task_index)));
            }
        }
        let mut old_cost = self.layout_cost(&nodes[..], graph, false);
        for iteration in 0..iterations {
            let temperature = 1.0 - (iteration as f32 / iterations as f32);
            let sample_index = rand::rng().random_range(0..nodes.len());
            let (sample_model, sample_task) = nodes.get(sample_index).unwrap();

            let distance = 2.0 + temperature * 15.0;
            let delta_x = rand::rng().random_range(-distance..distance);
            let delta_y = rand::rng().random_range(-distance..distance);
            self.position_mut(*sample_model, *sample_task)
                .unwrap()
                .position += Vector::new(delta_x, delta_y);

            let cost = self.layout_cost(&nodes[..], graph, false);
            if iteration % 25000 == 0 {
                println!("costs: {cost}")
            }
            if cost > old_cost {
                let cost_bound = temperature * 1.0;
                let permissible_cost = rand::rng().random_range(0.0..cost_bound);
                if cost > permissible_cost {
                    self.position_mut(*sample_model, *sample_task)
                        .unwrap()
                        .position -= Vector::new(delta_x, delta_y);
                }
            }
        }
        self.layout_cost(&nodes[..], graph, true);
    }

    fn layout_cost(
        &self,
        nodes: &[(usize, Option<usize>)],
        graph: &RepairGraph,
        print: bool,
    ) -> f32 {
        let print = |str: &str| {
            if print {
                println!("{str}")
            }
        };
        let costs = |divergence: f32, half_cost: f32| {
            if divergence <= 0.0 {
                0.0
            } else if divergence < half_cost {
                0.5 * divergence / half_cost
            } else {
                (0.5 + (divergence - half_cost) / (10.0 * half_cost)).min(1.0)
            }
        };

        let mut cost = 0.0;

        let mut min_y = f32::MAX;
        let mut max_y = 0.0f32;

        for (index_a, (a_model, a_task)) in nodes.iter().enumerate() {
            let mut node_costs = 0.0;

            let a_pos = self.position(*a_model, *a_task).unwrap();
            let a_x_start = a_pos.position.x - self.options.node_width * 0.5;
            let a_x_end = a_pos.position.x + self.options.node_width * 0.5;
            let a_y_start = a_pos.position.y;
            let a_y_end = a_pos.position.y + self.options.node_height;

            min_y = min_y.min(a_y_start);
            max_y = max_y.max(a_y_end);

            // Out of bounds:
            node_costs += costs(-a_y_start, 100.0) * 20.0;
            node_costs += costs(-self.options.width * 0.5 - a_x_start, 100.0) * 20.0;
            node_costs += costs(a_x_end - self.options.width * 0.5, 100.0) * 20.0;

            print(&format!("  Out of bounds costs: {node_costs}"));

            // Check overlaps:
            let mut overlap_cost = 0.0;
            for (index_b, (b_model, b_task)) in nodes.iter().enumerate() {
                if index_a != index_b {
                    let b_pos = self.position(*b_model, *b_task).unwrap();
                    let b_x_start = b_pos.position.x - self.options.node_width * 0.5;
                    let b_x_end = b_pos.position.x + self.options.node_width * 0.5;
                    let b_y_start = b_pos.position.y;
                    let b_y_end = b_pos.position.y + self.options.node_height;
                    let h_overlap = (a_x_end - b_x_start).min(b_x_end - a_x_start).max(0.0);
                    let v_overlap = (a_y_end - b_y_start).min(b_y_end - a_y_start).max(0.0);
                    let overlap = h_overlap * v_overlap;
                    print(&format!(
                        "    Overlap: {overlap} ({h_overlap} * {v_overlap})"
                    ));
                    overlap_cost += overlap / (self.options.node_width * self.options.node_height);
                }
            }
            node_costs += overlap_cost.min(1.0) * 0.5;
            print(&format!("  Overlap costs: {}", overlap_cost.min(1.0) * 0.5));

            let height_difference_cost = |y, parent_y| {
                let target_position =
                    parent_y + self.options.node_height + self.options.vertical_spacing;
                let delta: f32 = target_position - y;
                // println!(
                //     "Self at {y}, parent at {parent_y}, costs: {}",
                //     costs(delta.abs(), 300.0)
                // );
                costs(delta, 300.0)
            };
            // Check whether it is below children:
            match a_task {
                Some(task_index) => {
                    let model = &graph.nodes[*a_model];
                    let task = &model.tasks.tasks[*task_index];
                    for dependency in &task.dependencies {
                        let position = self.task_position(*a_model, *dependency).unwrap().position;
                        node_costs += height_difference_cost(a_pos.position.y, position.y);
                        print(&format!(
                            "    Height difference: {} (other: {})",
                            height_difference_cost(a_pos.position.y, position.y),
                            position.y
                        ));
                    }
                    if task.dependencies.is_empty() {
                        let model_position = self.model_position(*a_model).unwrap().position;
                        node_costs +=
                            height_difference_cost(a_pos.position.y, model_position.y) * 2.0;
                        print(&format!(
                            "    Height difference: {} (other: {})",
                            height_difference_cost(a_pos.position.y, model_position.y),
                            model_position.y
                        ));
                    }
                }
                None => {
                    print("    Node is a model node");
                }
            }
            cost += node_costs
        }
        cost /= nodes.len() as f32;

        // Make sure the first node starts at location 0.0
        cost += costs(min_y.abs(), 50.0);

        // Compress the graph as much as possible.
        cost += costs(
            max_y - min_y,
            nodes.len() as f32 * 1.5 * (self.options.node_height + self.options.vertical_spacing),
        );
        cost
    }

    pub fn model_position(&self, model: usize) -> Option<&LayoutNode> {
        self.layout.get(model).map(|m| &m.model_position)
    }
    pub fn model_position_mut(&mut self, model: usize) -> Option<&mut LayoutNode> {
        self.layout.get_mut(model).map(|m| &mut m.model_position)
    }

    pub fn task_position(&self, model: usize, task: usize) -> Option<&LayoutNode> {
        self.layout
            .get(model)
            .and_then(|m| m.task_positions.get(task))
    }

    pub fn task_position_mut(&mut self, model: usize, task: usize) -> Option<&mut LayoutNode> {
        self.layout
            .get_mut(model)
            .and_then(|m| m.task_positions.get_mut(task))
    }

    pub fn position(&self, model: usize, task: Option<usize>) -> Option<&LayoutNode> {
        match task {
            Some(task) => self.task_position(model, task),
            None => self.model_position(model),
        }
    }

    pub fn position_mut(&mut self, model: usize, task: Option<usize>) -> Option<&mut LayoutNode> {
        match task {
            Some(task) => self.task_position_mut(model, task),
            None => self.model_position_mut(model),
        }
    }
}

pub struct ModelLayout {
    pub model_position: LayoutNode,
    pub task_positions: Vec<LayoutNode>,
}

pub struct LayoutNode {
    pub position: Vector,
    stability: f32,
    pub window_state: WindowState,
}

impl LayoutNode {
    pub fn new(position: Vector, collapsible_sections: usize) -> Self {
        Self {
            position,
            stability: 0.0,
            window_state: WindowState::with_expanded_sections(collapsible_sections),
        }
    }
}
