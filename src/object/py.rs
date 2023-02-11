use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use pyo3::{prelude::*, pyclass::CompareOp};

#[pyclass]
#[derive(PartialEq, Eq)]
pub struct SourceSpan {
    #[pyo3(get, set)]
    filename: String,

    #[pyo3(get, set)]
    start_line: i32,

    #[pyo3(get, set)]
    end_line: i32,
}

#[pymethods]
impl SourceSpan {
    #[new]
    fn __new__(filename: String, start_line: i32, end_line: i32) -> Self {
        Self {
            filename,
            start_line,
            end_line,
        }
    }

    fn __str__(&self) -> String {
        format!("{}:{}-{}", self.filename, self.start_line, self.end_line)
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.filename.hash(&mut hasher);
        self.start_line.hash(&mut hasher);
        self.end_line.hash(&mut hasher);
        hasher.finish()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self == other),
            CompareOp::Ne => Ok(self != other),
            _ => unimplemented!("only equality exists for SourceSpan"),
        }
    }
}

impl From<super::SourceSpan> for SourceSpan {
    fn from(value: super::SourceSpan) -> Self {
        Self {
            filename: value.path.to_str().unwrap().to_string(),
            start_line: value.start as i32,
            end_line: value.end as i32,
        }
    }
}

#[pyclass]
pub struct ObjectPath {
    #[pyo3(get, set)]
    components: Vec<String>,
}

#[pymethods]
impl ObjectPath {
    #[new]
    fn __new__(components: Option<Vec<String>>) -> Self {
        if let Some(components) = components {
            Self { components }
        } else {
            Self {
                components: Vec::new(),
            }
        }
    }

    fn append_part(&mut self, part: String) {
        self.components.push(part);
    }

    fn __str__(&self) -> String {
        self.components.join(".")
    }
}

impl From<super::ObjectPath> for ObjectPath {
    fn from(value: super::ObjectPath) -> Self {
        Self {
            components: value.components,
        }
    }
}
