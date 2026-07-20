pub struct ParameterDescription {
    pub name: &'static str,
    pub values: ParameterType,
}

impl ParameterDescription {
    pub fn new(name: &'static str, values: ParameterType) -> Self {
        Self { name, values }
    }
}

pub enum ParameterType {
    Integer { min: Option<i64>, max: Option<i64> },
    Float { min: Option<f64>, max: Option<f64> },
    Boolean,
    Select { options: Vec<String> },
}

#[derive(Clone)]
pub enum ParameterValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Select(String),
}

impl ParameterValue {
    pub fn int(&self) -> Option<i64> {
        if let ParameterValue::Integer(val) = self {
            Some(*val)
        } else {
            None
        }
    }
    pub fn float(&self) -> Option<f64> {
        if let ParameterValue::Float(val) = self {
            Some(*val)
        } else {
            None
        }
    }
    pub fn bool(&self) -> Option<bool> {
        if let ParameterValue::Boolean(val) = self {
            Some(*val)
        } else {
            None
        }
    }
    pub fn select(&self) -> Option<&str> {
        if let ParameterValue::Select(val) = self {
            Some(val)
        } else {
            None
        }
    }
}
