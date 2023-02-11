use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use pyo3::{prelude::*, pyclass::CompareOp};

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    fn new(filename: String, start_line: i32, end_line: i32) -> Self {
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
        self.hash(&mut hasher);
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
#[derive(Clone, Debug)]
pub struct ObjectPath {
    #[pyo3(get, set)]
    components: Vec<String>,
}

#[pymethods]
impl ObjectPath {
    #[new]
    fn new(components: Option<Vec<String>>) -> Self {
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

#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub struct Object {
    #[pyo3(get, set)]
    source_span: SourceSpan,

    #[pyo3(get, set)]
    object_path: ObjectPath,

    #[pyo3(get, set)]
    children: HashMap<String, Object>,

    #[pyo3(get, set)]
    alt_counts: HashMap<String, i32>,

    #[pyo3(get, set)]
    name: String,
}

#[pymethods]
impl Object {
    #[new]
    fn new(source_span: SourceSpan, name: String, object_path: ObjectPath) -> Self {
        Self {
            source_span,
            object_path,
            name,
            children: HashMap::new(),
            alt_counts: HashMap::new(),
        }
    }

    fn __str__(&self) -> String {
        unimplemented!("Object is a base-class, no str representation")
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self == other),
            CompareOp::Ne => Ok(self != other),
            _ => unimplemented!("only equality exists for Object"),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.source_span == other.source_span && self.name == other.name
    }
}

impl Eq for Object {}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_span.hash(state);
        self.name.hash(state);
    }
}

#[pyclass(extends=Object)]
#[derive(Debug, Clone)]
pub struct AltObject {
    #[pyo3(get, set)]
    alt_name: String,

    #[pyo3(get, set)]
    sub_ob: Object,
}

#[pymethods]
impl AltObject {
    #[new]
    fn new(
        source_span: SourceSpan,
        name: String,
        object_path: ObjectPath,
        sub_ob: Object,
        alt_cnt: i32,
    ) -> (Self, Object) {
        let alt_name = format!("{}#{}", name, alt_cnt);
        let ob = Object::new(source_span, alt_name.clone(), object_path);
        let alt = AltObject { alt_name, sub_ob };
        (alt, ob)
    }
}

#[pyclass(extends=Object)]
#[derive(Clone, Debug)]
pub struct Module;

#[pymethods]
impl Module {
    #[new]
    fn new(source_span: SourceSpan, name: String, object_path: ObjectPath) -> (Self, Object) {
        (Self {}, Object::new(source_span, name, object_path))
    }

    fn __str__(&self) -> String {
        "module".into()
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}
