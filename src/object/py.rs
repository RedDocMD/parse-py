use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use pyo3::{exceptions::PyValueError, prelude::*, pyclass::CompareOp};

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    filename: String,
    start_line: i32,
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

    formatted_path: String,
}

#[pymethods]
impl ObjectPath {
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
            formatted_path: value.to_string(),
            components: value.components,
        }
    }
}

#[pyclass(subclass, get_all, set_all)]
#[derive(Clone, Debug)]
pub struct Object {
    source_span: SourceSpan,
    object_path: ObjectPath,
    children: HashMap<String, PyObject>,
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

#[pyclass(extends=Object, get_all, set_all)]
#[derive(Debug, Clone)]
pub struct AltObject {
    alt_name: String,
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

#[pyclass(extends=Object)]
#[derive(Clone, Debug)]
pub struct Class;

#[pymethods]
impl Class {
    #[new]
    fn new(source_span: SourceSpan, name: String, object_path: ObjectPath) -> (Self, Object) {
        (Self {}, Object::new(source_span, name, object_path))
    }

    fn __str__(&self) -> String {
        "class".into()
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

#[pyclass]
#[derive(Debug, Clone, Copy)]
pub enum FormalParamKind {
    POSONLY = 0,
    NORMAL = 1,
    KWONLY = 2,
}

impl From<super::FormalParamKind> for FormalParamKind {
    fn from(value: super::FormalParamKind) -> Self {
        match value {
            super::FormalParamKind::PosOnly => Self::POSONLY,
            super::FormalParamKind::KwOnly => Self::KWONLY,
            super::FormalParamKind::Normal => Self::NORMAL,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone)]
pub struct FormalParam {
    name: String,
    has_default: bool,
    kind: FormalParamKind,
}

#[pymethods]
impl FormalParam {
    #[new]
    fn new(name: String, has_default: bool, kind: FormalParamKind) -> Self {
        Self {
            name,
            has_default,
            kind,
        }
    }
}

impl From<super::FormalParam> for FormalParam {
    fn from(value: super::FormalParam) -> Self {
        Self {
            name: value.name,
            has_default: value.has_default,
            kind: value.kind.into(),
        }
    }
}

// FIXME: Add stmts
#[pyclass(extends=Object)]
#[derive(Clone, Debug)]
pub struct Function {
    formal_params: Vec<FormalParam>,
    kwarg: Option<String>,
    formatted_args: String,
}

#[pymethods]
impl Function {
    fn has_kwargs_dict(&self) -> bool {
        self.kwarg.is_some()
    }

    fn get_kwargs_name(&self) -> PyResult<String> {
        self.kwarg
            .clone()
            .ok_or_else(|| PyValueError::new_err("fn has not got keyword arguments"))
    }

    fn get_formal_params(&self) -> Vec<FormalParam> {
        self.formal_params.clone()
    }

    fn __repr__(self_: PyRef<'_, Self>) -> String {
        Function::__str__(self_)
    }

    fn __str__(self_: PyRef<'_, Self>) -> String {
        let super_ = self_.as_ref();
        format!(
            "function {}({})",
            super_.object_path.formatted_path, self_.formatted_args
        )
    }
}
