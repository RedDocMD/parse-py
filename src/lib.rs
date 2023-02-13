use pyo3::prelude::*;

pub mod object;
pub mod project;

#[pymodule]
fn parse_py(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<object::py::SourceSpan>()?;
    m.add_class::<object::py::ObjectPath>()?;
    m.add_class::<object::py::Object>()?;
    m.add_class::<object::py::AltObject>()?;
    m.add_class::<object::py::Module>()?;
    m.add_class::<object::py::Class>()?;
    m.add_class::<object::py::FormalParamKind>()?;
    m.add_class::<object::py::Function>()?;
    Ok(())
}
