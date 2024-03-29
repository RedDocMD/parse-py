use std::{
    collections::{
        hash_map::{DefaultHasher, IntoIter as HashMapIntoIter},
        HashMap,
    },
    hash::{Hash, Hasher},
    path::PathBuf,
};

use pyo3::{exceptions::PyRuntimeError, prelude::*, pyclass::CompareOp};

use crate::object::py::module_to_py;

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    filename: String,
    start_line: i32,
}

#[pymethods]
impl Position {
    #[new]
    fn new(filename: String, start_line: i32) -> Self {
        Self {
            filename,
            start_line,
        }
    }

    fn __str__(&self) -> String {
        format!("{}:{}", self.filename, self.start_line)
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
            _ => unimplemented!("only equality exists for Position"),
        }
    }
}

#[pyclass]
pub struct ObjectDb {
    db: HashMap<Position, PyObject>,
}

#[pymethods]
impl ObjectDb {
    fn __getitem__(&self, pos: &Position) -> &PyObject {
        &self.db[pos]
    }

    fn __len__(&self) -> usize {
        self.db.len()
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<DbIter>> {
        let iter = DbIter {
            inner: slf.db.clone().into_iter(),
        };
        Py::new(slf.py(), iter)
    }

    // TODO: Implement items()
    // TODO: Implement values()
    // TODO: Implement has_ob()
    // TODO: Implement lookup_fn()
}

#[pyclass]
struct DbIter {
    inner: HashMapIntoIter<Position, PyObject>,
}

#[pymethods]
impl DbIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(Position, PyObject)> {
        slf.inner.next()
    }
}

impl From<super::ProjectError> for PyErr {
    fn from(value: super::ProjectError) -> Self {
        let msg = value.to_string();
        PyRuntimeError::new_err(msg)
    }
}

#[pyfunction]
#[pyo3(signature = (path))]
pub fn module_from_dir(py: Python, path: String) -> PyResult<&PyAny> {
    let path = PathBuf::from(path);
    let project = super::Project::create(path)?;
    let module = module_to_py(py, project.root_ob)?;
    Ok(module)
}
