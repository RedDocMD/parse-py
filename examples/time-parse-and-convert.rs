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
    println!("{}", name);

    let path = PathBuf::from(path);

    let parse_start = Instant::now();
    let project = Project::create(path).unwrap();
    let parse_end = Instant::now();
    println!("  Parse => {}ms", (parse_end - parse_start).as_millis());

    Python::with_gil(|py| {
        let translate_start = Instant::now();
        let _mod_py = module_to_py(py, project.root_ob).unwrap();
        let translate_end = Instant::now();
        println!(
            "  Translate => {}ms",
            (translate_end - translate_start).as_millis()
        );
    });
}
