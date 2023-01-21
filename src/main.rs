use std::{error::Error, fs, path::Path};

use rustpython_parser::parser;

fn main() {
    const PROJ_PATH: &str = "/home/dknite/work/python/sympy/sympy";
    let tot_stmts = parse_dir(PROJ_PATH).unwrap();
    println!("tot_stmts = {}", tot_stmts);
}

fn parse_dir<P: AsRef<Path>>(path: P) -> Result<usize, Box<dyn Error>> {
    let mut cnt = 0;
    for p in fs::read_dir(&path)? {
        let p = p?;
        let name = p.file_name();
        let name = name.to_str().unwrap();

        if name.ends_with(".py") {
            let content = fs::read(p.path())?;
            let content_str = std::str::from_utf8(&content)?;
            let code = parser::parse_program(&content_str, &name)?;
            cnt += code.len();
        } else if p.metadata()?.is_dir() {
            cnt += parse_dir(&p.path())?;
        }
    }
    Ok(cnt)
}
