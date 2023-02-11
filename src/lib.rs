use pyo3::prelude::*;

pub mod object;
pub mod project;

#[pymodule]
fn parse_py(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<object::py::SourceSpan>()?;
    Ok(())
}
