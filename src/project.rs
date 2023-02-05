use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::object::{Module, ModuleCreator, Object, ObjectPath};

pub struct Position {
    filename: PathBuf,
    start: usize,
}

pub type ObjectDb = HashMap<Position, Object>;

pub struct Project {
    root: PathBuf,
    // db: ObjectDb,
    // kw_fns: ObjectDb,
    root_ob: Module,
}

impl Project {
    pub fn create(root: PathBuf) -> Result<Self> {
        let root_ob = module_from_dir(ObjectPath::default(), root.clone())?
            .ok_or_else(|| ProjectError::EmptyRoot(root.clone()))?;
        Ok(Self { root_ob, root })
    }
}

fn db_from_object(ob: Object) -> ObjectDb {
    todo!()
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("OsString was not valid UTF-8")]
    OsStringNotUtf8,

    #[error("parse error: {0}")]
    Parse(#[from] rustpython_parser::error::ParseError),

    #[error("no Python module in {}", .0.display())]
    EmptyRoot(PathBuf),
}

pub type Result<T> = std::result::Result<T, ProjectError>;

fn module_from_dir(par_path: ObjectPath, dir: PathBuf) -> Result<Option<Module>> {
    let drc = DirChildren::create(&dir)?;
    let Some(init) = drc.init else {
        return Ok(None);
    };

    let mut main_mod = mod_from_file(init, par_path.clone())?;
    let mut new_path = par_path;
    new_path.append_part(main_mod.name().to_string());

    for file in drc.files {
        let child_mod = mod_from_file(file, new_path.clone())?;
        main_mod.append_child(Object::Module(child_mod));
    }
    for dir in drc.dirs {
        let child_ob = module_from_dir(new_path.clone(), dir)?;
        if let Some(child_ob) = child_ob {
            main_mod.append_child(Object::Module(child_ob));
        }
    }

    Ok(Some(main_mod))
}

fn mod_from_file(path: PathBuf, par_path: ObjectPath) -> Result<Module> {
    let code = std::fs::read_to_string(&path)?;
    let line_cnt = code.bytes().filter(|c| c == &b'\n').count() + 1;
    let stmts = rustpython_parser::parser::parse_program(
        &code,
        path.to_str().ok_or(ProjectError::OsStringNotUtf8)?,
    )?;
    Ok(ModuleCreator::new(path.to_path_buf(), line_cnt, par_path).create(stmts))
}

struct DirChildren {
    init: Option<PathBuf>,
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

impl DirChildren {
    fn create(path: &Path) -> Result<Self> {
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let mut init = None;

        for entry in path.read_dir()? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or(ProjectError::OsStringNotUtf8)?;
            let kind = entry.file_type()?;
            let entry_path = entry.path();
            if kind.is_dir() && name != "__pycache__" {
                dirs.push(entry_path);
            } else if kind.is_file() {
                if !name.ends_with(".py") {
                    continue;
                }
                if name == "__init__.py" {
                    init = Some(entry_path);
                } else {
                    files.push(entry_path);
                }
            }
        }

        Ok(Self { files, dirs, init })
    }
}
