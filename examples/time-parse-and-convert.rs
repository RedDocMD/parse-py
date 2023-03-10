use std::{path::PathBuf, time::Instant};

use parse_py::{object::py::module_to_py, project::Project};
use pyo3::Python;

fn main() {
    let projects = vec![
        ("requests", "projects/requests/requests"),
        ("sympy", "projects/sympy/sympy"),
        ("pandas", "projects/pandas/pandas"),
    ];
    pyo3::prepare_freethreaded_python();
    for (name, path) in projects {
        do_parse(name, path);
    }
}

fn do_parse(name: &str, path: &str) {
    let path = PathBuf::from(path);
    let start = Instant::now();
    let project = Project::create(path).unwrap();
    Python::with_gil(|py| {
        let _mod_py = module_to_py(py, project.root_ob).unwrap();
    });
    let end = Instant::now();
    let duration = end - start;
    println!("{} => {}ms", name, duration.as_millis());
}
