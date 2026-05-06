#[derive(Copy, Clone, Debug)]
pub enum PermissibleBounds {
    IntegerRange { min: i64, max: i64 },
    FloatRange { min: f64, max: f64 },
    Unknown,
}

impl PermissibleBounds {
    pub fn add_min_int_constraint(&mut self, constraint: i64) {
        match self {
            PermissibleBounds::IntegerRange { min, .. } => *min = (*min).max(constraint),
            _ => panic!("Can only add integer min bound to integer range"),
        }
    }
    pub fn add_max_int_constraint(&mut self, constraint: i64) {
        match self {
            PermissibleBounds::IntegerRange { max, .. } => *max = (*max).min(constraint),
            _ => panic!("Can only add integer max bound to integer range"),
        }
    }

    pub fn min_int(&self) -> i64 {
        match self {
            PermissibleBounds::IntegerRange { min, .. } => *min,
            _ => panic!("Unknown integer bounds"),
        }
    }
    pub fn max_int(&self) -> i64 {
        match self {
            PermissibleBounds::IntegerRange { max, .. } => *max,
            _ => panic!("Unknown integer bounds"),
        }
    }

    pub fn apply_integer_operation<F: Fn(i64) -> i64>(self, op: F) -> Self {
        match self {
            PermissibleBounds::IntegerRange { min, max } => {
                let min = op(min);
                let max = op(max);
                PermissibleBounds::IntegerRange {
                    min: min.min(max),
                    max: min.max(max),
                }
            }
            _ => PermissibleBounds::Unknown,
        }
    }
    pub fn apply_integer_operation_with_rounding<F: Fn(i64) -> f64>(self, op: F) -> Self {
        match self {
            PermissibleBounds::IntegerRange { min, max } => {
                let min = op(min);
                let max = op(max);
                let (min, max) = (min.min(max).ceil() as i64, min.max(max).floor() as i64);
                PermissibleBounds::IntegerRange { min, max }
            }
            _ => PermissibleBounds::Unknown,
        }
    }
    pub fn apply_numeric_operation<FI: Fn(i64) -> i64, FF: Fn(f64) -> f64>(
        self,
        op_int: FI,
        op_float: FF,
    ) -> Self {
        match self {
            PermissibleBounds::IntegerRange { min, max } => {
                let min = op_int(min);
                let max = op_int(max);
                PermissibleBounds::IntegerRange {
                    min: min.min(max),
                    max: min.max(max),
                }
            }
            PermissibleBounds::FloatRange { min, max } => {
                let min = op_float(min);
                let max = op_float(max);
                PermissibleBounds::FloatRange {
                    min: min.min(max),
                    max: min.max(max),
                }
            }

            PermissibleBounds::Unknown => PermissibleBounds::Unknown,
        }
    }
}
