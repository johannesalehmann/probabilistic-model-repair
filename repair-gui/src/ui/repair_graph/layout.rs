use crate::ui::repair_graph::window_builder::WindowState;
use repair_lib::repair_graph::RepairGraph;

pub struct LayoutOptions {
    pub node_width: f32,
    pub slots_per_row: usize,
}

impl LayoutOptions {
    pub fn new() -> Self {
        Self {
            node_width: 300.0,
            slots_per_row: 3,
        }
    }
}

pub struct RepairGraphLayout {
    pub rows: Vec<LayoutRow>,
    pub node_to_position: Vec<ModelLayoutInfo>,
    pub options: LayoutOptions,
}

impl RepairGraphLayout {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            node_to_position: Vec::new(),
            options: LayoutOptions::new(),
        }
    }

    fn create_position(
        &mut self,
        start_row: usize,
        start_column: usize,
        node_index: (usize, Option<usize>),
    ) -> (usize, usize) {
        let mut row = start_row;
        loop {
            while self.rows.len() <= row {
                self.rows.push(LayoutRow {
                    entries: vec![None; self.options.slots_per_row],
                });
            }
            for delta in 0..self.options.slots_per_row {
                for side in [-1, 1] {
                    let column = (start_column as i64 + delta as i64 * side);
                    if column >= 0 && column < self.options.slots_per_row as i64 {
                        let column = column as usize;
                        if self.rows[row].entries[column].is_none() {
                            self.rows[row].entries[column] = Some(LayoutEntry {
                                node_index,
                                window_state: WindowState::with_expanded_sections(16),
                            });
                            if let Some(task_index) = node_index.1 {
                                assert!(self.node_to_position.len() > node_index.0);
                                assert_eq!(
                                    self.node_to_position[node_index.0].task_positions.len(),
                                    task_index
                                );
                                self.node_to_position[node_index.0]
                                    .task_positions
                                    .push(LayoutGridPosition { column, row })
                            } else {
                                assert_eq!(self.node_to_position.len(), node_index.0);
                                self.node_to_position.push(ModelLayoutInfo {
                                    model_position: LayoutGridPosition { column, row },
                                    task_positions: Vec::new(),
                                })
                            }
                            return (row, column);
                        }
                    }
                }
            }
            row += 1;
        }
    }

    pub fn update_for_graph(&mut self, graph: &RepairGraph) {
        for (model_index, model_node) in graph.nodes.iter().enumerate() {
            if self.node_to_position.len() <= model_index {
                let (start_row, start_column) = model_node
                    .parent
                    .as_ref()
                    .and_then(|p| self.task_position(p.model_index, p.task_index))
                    .map(|p| (p.row + 1, p.column))
                    .unwrap_or((0, self.options.slots_per_row / 2));

                self.create_position(start_row, start_column, (model_index, None));
            }
            for (task_index, task_node) in model_node.tasks.tasks.iter().enumerate() {
                if self.node_to_position[model_index].task_positions.len() <= task_index {
                    let (start_row, start_column) = task_node
                        .dependencies
                        .get(0)
                        .and_then(|i| self.task_position(model_index, *i))
                        .or(self.model_position(model_index))
                        .map(|p| (p.row + 1, p.column))
                        .unwrap_or((0, self.options.slots_per_row / 2));
                    self.create_position(start_row, start_column, (model_index, Some(task_index)));
                }
            }
        }
    }

    pub fn model_position(&self, model: usize) -> Option<&LayoutGridPosition> {
        self.node_to_position.get(model).map(|m| &m.model_position)
    }
    pub fn model_position_mut(&mut self, model: usize) -> Option<&mut LayoutGridPosition> {
        self.node_to_position
            .get_mut(model)
            .map(|m| &mut m.model_position)
    }

    pub fn task_position(&self, model: usize, task: usize) -> Option<&LayoutGridPosition> {
        self.node_to_position
            .get(model)
            .and_then(|m| m.task_positions.get(task))
    }

    pub fn task_position_mut(
        &mut self,
        model: usize,
        task: usize,
    ) -> Option<&mut LayoutGridPosition> {
        self.node_to_position
            .get_mut(model)
            .and_then(|m| m.task_positions.get_mut(task))
    }

    pub fn position(&self, model: usize, task: Option<usize>) -> Option<&LayoutGridPosition> {
        match task {
            Some(task) => self.task_position(model, task),
            None => self.model_position(model),
        }
    }

    pub fn position_mut(
        &mut self,
        model: usize,
        task: Option<usize>,
    ) -> Option<&mut LayoutGridPosition> {
        match task {
            Some(task) => self.task_position_mut(model, task),
            None => self.model_position_mut(model),
        }
    }
}

pub struct LayoutRow {
    pub entries: Vec<Option<LayoutEntry>>,
}

#[derive(Clone)]
pub struct LayoutEntry {
    pub node_index: (usize, Option<usize>),
    pub window_state: WindowState,
}

#[derive(Default)]
pub struct LayoutGridPosition {
    pub column: usize,
    pub row: usize,
}

#[derive(Default)]
pub struct ModelLayoutInfo {
    pub model_position: LayoutGridPosition,
    pub task_positions: Vec<LayoutGridPosition>,
}
