use std::{path::PathBuf, time::Instant};

use parse_py::project::Project;

fn main() {
    let projects = vec![
        ("requests", "projects/requests/requests"),
        ("sympy", "projects/sympy/sympy"),
        ("pandas", "projects/pandas/pandas"),
    ];
    for (name, path) in projects {
        do_parse(name, path);
    }
}

fn do_parse(name: &str, path: &str) {
    let path = PathBuf::from(path);
    let start = Instant::now();
    let _project = Project::create(path).unwrap();
    let end = Instant::now();
    let duration = end - start;
    println!("{} => {}ms", name, duration.as_millis());
}
