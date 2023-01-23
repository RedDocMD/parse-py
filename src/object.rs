use std::{
    collections::HashMap,
    fmt::{self, Display},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use rustpython_parser::ast::{Arg, Arguments, StmtKind};

/// Represents a span in a Python source file.
/// This span typically denotes something, like a function or class.
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct SourceSpan {
    path: PathBuf,
    start: i32,
    end: i32,
}

impl Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}-{}", self.path.display(), self.start, self.end)
    }
}

impl SourceSpan {
    pub fn new(path: PathBuf, start: i32, end: i32) -> Self {
        Self { path, start, end }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn start(&self) -> i32 {
        self.start
    }

    pub fn end(&self) -> i32 {
        self.end
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

    pub fn replaece_name(&mut self, new_name: String) {
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
    name: String,
    has_default: bool,
    kind: FormalParamKind,
}

/// Represents a function in Python, either top-level,
/// or part of a class.
#[derive(Debug, Clone)]
pub struct Function {
    data: ObjectData,
    args: Arguments,
    stmts: Vec<StmtKind>,
}

fn arg_names(args: &[Arg]) -> Vec<String> {
    args.into_iter().map(|arg| arg.node.arg.clone()).collect()
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
        todo!()
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
        obj_path.replaece_name(alt_name);
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
