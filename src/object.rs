use std::{
    collections::HashMap,
    fmt::{self, Display},
    hash::{Hash, Hasher},
    os::unix::prelude::OsStrExt,
    path::{Component, Path, PathBuf},
};

use rustpython_parser::ast::{Arg, Arguments, ExcepthandlerKind, Location, Stmt, StmtKind};

pub mod py;

/// Represents a span in a Python source file.
/// This span typically denotes something, like a function or class.
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct SourceSpan {
    path: PathBuf,
    start: usize,
    end: usize,
}

// Represents a Python source element by its starting position
// and filename.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    filename: PathBuf,
    start: usize,
}

impl Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}-{}", self.path.display(), self.start, self.end)
    }
}

impl SourceSpan {
    pub fn new(path: PathBuf, start: usize, end: usize) -> Self {
        Self { path, start, end }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

impl From<SourceSpan> for Position {
    fn from(span: SourceSpan) -> Self {
        Self {
            filename: span.path,
            start: span.start,
        }
    }
}

/// This represents a fully cannonical path of some "thing" in Python,
/// such as `os.path.join`, which is a function.
#[derive(Clone, Debug, Default)]
pub struct ObjectPath {
    components: Vec<String>,
}

impl ObjectPath {
    pub fn new(components: Vec<String>) -> Self {
        Self { components }
    }

    pub fn append_part(&mut self, part: String) {
        self.components.push(part)
    }

    pub fn name(&self) -> &str {
        self.components.last().unwrap()
    }

    pub fn replace_name(&mut self, new_name: String) {
        *self.components.last_mut().unwrap() = new_name;
    }
}

impl Display for ObjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, comp) in self.components.iter().enumerate() {
            if i != 0 {
                write!(f, ".")?;
            }
            write!(f, "{}", comp)?;
        }
        Ok(())
    }
}

/// Represents the common data in all variants of [`Object`].
#[derive(Debug, Clone)]
pub struct ObjectData {
    span: SourceSpan,
    children: HashMap<String, Object>,
    alt_cnts: HashMap<String, i32>,
    obj_path: ObjectPath,
}

impl ObjectData {
    pub fn new(span: SourceSpan, obj_path: ObjectPath) -> Self {
        Self {
            span,
            children: HashMap::new(),
            alt_cnts: HashMap::new(),
            obj_path,
        }
    }

    pub fn name(&self) -> &str {
        self.obj_path.name()
    }

    pub fn append_child(&mut self, name: String, child: Object) {
        let (name, child) = if self.children.contains_key(&name) {
            let entry = self.alt_cnts.entry(name).or_default();
            *entry += 1;
            let alt_cnt = *entry;
            let span = child.data().span.clone();
            let obj_path = child.data().obj_path.clone();
            let alt_ob = AltObject::new(span, obj_path, child, alt_cnt);
            let name = alt_ob.data.name().to_string();
            (name, Object::AltObject(alt_ob))
        } else {
            (name, child)
        };
        self.children.insert(name, child);
    }

    pub fn append_children(&mut self, children: Vec<Object>) {
        for child in children {
            let name = child.data().name().to_string();
            self.append_child(name, child);
        }
    }

    pub fn position(&self) -> Position {
        self.span.clone().into()
    }
}

impl PartialEq for ObjectData {
    fn eq(&self, other: &Self) -> bool {
        self.span == other.span && self.name() == other.name()
    }
}

impl Hash for ObjectData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.span.hash(state);
        self.name().hash(state);
    }
}

/// Represents a Python module, which is basically all the stuff
/// in a file.
#[derive(Debug, Clone)]
pub struct Module {
    data: ObjectData,
}

impl Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mod {}", self.data.name())
    }
}

impl Module {
    pub fn name(&self) -> &str {
        self.data.name()
    }

    pub fn append_child(&mut self, child: Object) {
        self.data
            .append_child(child.data().name().to_string(), child);
    }
}

/// Represents a Python class.
#[derive(Debug, Clone)]
pub struct Class {
    data: ObjectData,
}

impl Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "class {}", self.data.name())
    }
}

/// The kind of a formal parameter of a function.
#[derive(Debug, Clone, Copy)]
pub enum FormalParamKind {
    PosOnly,
    KwOnly,
    Normal,
}

/// Denotes a formal parameter of a function.
#[derive(Debug, Clone)]
pub struct FormalParam {
    pub name: String,
    pub has_default: bool,
    pub kind: FormalParamKind,
}

/// Represents a function in Python, either top-level,
/// or part of a class.
#[derive(Debug, Clone)]
pub struct Function {
    data: ObjectData,
    args: Arguments,
    stmts: HashMap<usize, StmtKind>,
}

impl Function {
    pub fn has_kwargs_dict(&self) -> bool {
        self.args.kwarg.is_some()
    }

    pub fn kwargs_name(&self) -> String {
        assert!(self.has_kwargs_dict());
        self.args.kwarg.as_ref().unwrap().node.arg.to_string()
    }

    pub fn formal_params(&self) -> Vec<FormalParam> {
        fn arg_names(args: &[Arg]) -> Vec<String> {
            args.iter().map(|arg| arg.node.arg.clone()).collect()
        }

        let posonly = arg_names(&self.args.posonlyargs);
        let normal = arg_names(&self.args.args);
        let kwonly = arg_names(&self.args.kwonlyargs);

        let def_cnt = self.args.defaults.len();
        let norm_def_cnt = normal.len().min(def_cnt);
        let posonly_def_cnt = posonly.len().min(def_cnt - norm_def_cnt);
        let kwonly_def_cnt = self.args.kw_defaults.len();

        let mut params = Vec::new();

        for (i, arg) in posonly.iter().enumerate() {
            let has_default = i >= (posonly.len() - posonly_def_cnt);
            params.push(FormalParam {
                name: arg.to_string(),
                has_default,
                kind: FormalParamKind::PosOnly,
            });
        }
        for (i, arg) in normal.iter().enumerate() {
            let has_default = i >= (normal.len() - norm_def_cnt);
            params.push(FormalParam {
                name: arg.to_string(),
                has_default,
                kind: FormalParamKind::Normal,
            });
        }
        for (i, arg) in kwonly.iter().enumerate() {
            let has_default = i > (kwonly.len() - kwonly_def_cnt);
            params.push(FormalParam {
                name: arg.to_string(),
                has_default,
                kind: FormalParamKind::KwOnly,
            })
        }

        params
    }

    pub fn format_args(&self) -> String {
        fn make_arg_list(args: &[Arg]) -> String {
            let mut list = String::new();
            for (i, arg) in args.iter().enumerate() {
                if i != 0 {
                    list.push_str(", ");
                }
                list.push_str(&arg.node.arg);
            }
            list
        }

        let args = make_arg_list(&self.args.args);
        let posonly = make_arg_list(&self.args.posonlyargs);
        let kwonly = make_arg_list(&self.args.kwonlyargs);

        let mut out = String::new();
        if !posonly.is_empty() {
            out.push_str(&posonly);
            out.push('/');
        }
        out.push_str(&args);
        if let Some(vararg) = &self.args.vararg {
            if (!out.is_empty() && out.as_bytes().last().unwrap() != &b'/') || !out.is_empty() {
                out.push_str("/ ");
            }
            out.push('*');
            out.push_str(&vararg.node.arg);
            if !kwonly.is_empty() {
                out.push_str(", ");
                out.push_str(&kwonly);
            }
        }
        if let Some(kwarg) = &self.args.kwarg {
            if (!out.is_empty() && out.as_bytes().last().unwrap() != &b'/') || !out.is_empty() {
                out.push_str(", ");
            }
            out.push_str("**");
            out.push_str(&kwarg.node.arg);
        }

        out
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "function {}({})", self.data.obj_path, self.format_args())
    }
}

/// Used for representing alternate forms of the same bit of code.
///
/// For example, consider the following code:
/// ```py
/// if foo == 'hello':
///     def bar():
///         print('forrest')
/// else:
///     def bar():
///         print('gump')
/// ```
/// Here, the first bar() will be the main object.
/// The second bar() will be represented as an alt-object.
#[derive(Debug, Clone)]
pub struct AltObject {
    data: ObjectData,
    sub_ob: Box<Object>,
}

impl AltObject {
    pub fn new(
        source_span: SourceSpan,
        mut obj_path: ObjectPath,
        sub_ob: Object,
        alt_cnt: i32,
    ) -> Self {
        let alt_name = format!("{}#{}", obj_path.name(), alt_cnt);
        obj_path.replace_name(alt_name);
        let data = ObjectData::new(source_span, obj_path);
        Self {
            data,
            sub_ob: Box::new(sub_ob),
        }
    }
}

/// This is an entity in Python, such as module, class or function.
#[derive(Debug, Clone)]
pub enum Object {
    Module(Module),
    Class(Class),
    Function(Function),
    AltObject(AltObject),
}

impl Object {
    pub fn data(&self) -> &ObjectData {
        match self {
            Object::Module(m) => &m.data,
            Object::Class(c) => &c.data,
            Object::Function(f) => &f.data,
            Object::AltObject(a) => &a.data,
        }
    }

    pub fn into_data(self) -> ObjectData {
        match self {
            Object::Module(m) => m.data,
            Object::Class(c) => c.data,
            Object::Function(f) => f.data,
            Object::AltObject(a) => a.data,
        }
    }

    pub fn into_children(self) -> impl Iterator<Item = Object> {
        self.into_data().children.into_values()
    }

    pub fn ob_type(&self) -> &'static str {
        match self {
            Object::Module(_) => "mod",
            Object::Class(_) => "class",
            Object::Function(_) => "func",
            Object::AltObject(a) => a.sub_ob.ob_type(),
        }
    }

    fn _dump_tree(&self, level: usize) {
        let padding = "  ".repeat(level);
        println!(
            "{}{} ({}) => {}:{}",
            padding,
            self.data().name(),
            self.ob_type(),
            self.data().span.path.display(),
            self.data().span.start
        );
        for child in self.data().children.values() {
            child._dump_tree(level + 1);
        }
    }

    pub fn dump_tree(&self) {
        self._dump_tree(0)
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.data() == other.data() && self.ob_type() == other.ob_type()
    }
}

impl Eq for Object {}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data().hash(state);
        self.ob_type().hash(state);
    }
}

pub struct ModuleCreator {
    filename: PathBuf,
    line_cnt: usize,
    par_path: ObjectPath,
}

impl ModuleCreator {
    pub fn new(filename: PathBuf, line_cnt: usize, par_path: ObjectPath) -> Self {
        ModuleCreator {
            filename,
            line_cnt,
            par_path,
        }
    }

    pub fn create(self, stmts: Vec<Stmt>) -> Module {
        let mod_path = self.mod_path();
        let children = objects_from_stmts(stmts, &mod_path, &self.filename);
        let mod_span = SourceSpan::new(self.filename, 0, self.line_cnt);
        let mut mod_data = ObjectData::new(mod_span, mod_path);
        mod_data.append_children(children);
        Module { data: mod_data }
    }

    fn mod_path(&self) -> ObjectPath {
        let mut mod_path = self.par_path.clone();
        mod_path.append_part(self.mod_name());
        mod_path
    }

    fn mod_name(&self) -> String {
        let mut parts = self.filename.components().rev();
        let last = parts.next().unwrap();
        if let Component::Normal(last) = last {
            if last.as_bytes() == b"__init__.py" {
                let par = parts.next().unwrap();
                if let Component::Normal(par) = par {
                    par.to_os_string().into_string().unwrap()
                } else {
                    unreachable!("mod path must have parent");
                }
            } else {
                last.to_os_string().into_string().unwrap()
            }
        } else {
            unreachable!("mod path must have a filename");
        }
    }
}

fn extract_statements_from_body(stmts: Vec<Stmt>) -> HashMap<usize, StmtKind> {
    let mut stmts_map = HashMap::new();
    for stmt in stmts {
        stmts_map.extend(extract_statement(stmt));
    }
    stmts_map
}

fn extract_statement(stmt: Stmt) -> HashMap<usize, StmtKind> {
    let node = stmt.node;
    let mut stmts = HashMap::from([(stmt.location.row(), node.clone())]);
    match node {
        // Don't recurse into function or class definitions, that is handled else-where
        StmtKind::FunctionDef { .. } => stmts.clear(),
        StmtKind::AsyncFunctionDef { .. } => stmts.clear(),
        StmtKind::ClassDef { .. } => stmts.clear(),
        // For the rest, recurse
        StmtKind::For { body, .. } => stmts.extend(extract_statements_from_body(body)),
        StmtKind::AsyncFor { body, .. } => stmts.extend(extract_statements_from_body(body)),
        StmtKind::While { body, .. } => stmts.extend(extract_statements_from_body(body)),
        StmtKind::If { body, .. } => stmts.extend(extract_statements_from_body(body)),
        StmtKind::With { body, .. } => stmts.extend(extract_statements_from_body(body)),
        StmtKind::AsyncWith { body, .. } => stmts.extend(extract_statements_from_body(body)),
        StmtKind::Match { cases, .. } => {
            for cs in cases {
                stmts.extend(extract_statements_from_body(cs.body));
            }
        }
        StmtKind::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            for b in [body, orelse, finalbody] {
                stmts.extend(extract_statements_from_body(b));
            }
            for h in handlers {
                match h.node {
                    ExcepthandlerKind::ExceptHandler { body, .. } => {
                        stmts.extend(extract_statements_from_body(body));
                    }
                }
            }
        }
        _ => {}
    }
    stmts
}

fn objects_from_stmts(stmts: Vec<Stmt>, par_path: &ObjectPath, file_path: &Path) -> Vec<Object> {
    let make_span = |loc: Location, end_loc: Option<Location>| {
        let start = loc.row();
        let end = end_loc.unwrap().row();
        SourceSpan::new(file_path.to_path_buf(), start, end)
    };
    let make_path = |name: String| {
        let mut path = par_path.clone();
        path.append_part(name);
        path
    };

    let mut objects = Vec::new();
    for stmt in stmts {
        let kind = stmt.node;
        match kind {
            StmtKind::ClassDef { name, body, .. } => {
                let class_path = make_path(name);
                let class_span = make_span(stmt.location, stmt.end_location);

                let children = objects_from_stmts(body, &class_path, file_path);
                let mut class_data = ObjectData::new(class_span, class_path);
                class_data.append_children(children);
                let class = Class { data: class_data };
                objects.push(Object::Class(class));
            }
            StmtKind::FunctionDef {
                name, args, body, ..
            } => {
                let func_path = make_path(name);
                let func_span = make_span(stmt.location, stmt.end_location);

                let children = objects_from_stmts(body.clone(), &func_path, file_path);
                let stmts = extract_statements_from_body(body);
                let mut func_data = ObjectData::new(func_span, func_path);
                func_data.append_children(children);

                let func = Function {
                    data: func_data,
                    args: *args,
                    stmts,
                };
                objects.push(Object::Function(func));
            }
            // TODO: Handle async function
            _ => {}
        }
    }
    objects
}
