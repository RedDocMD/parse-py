use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use itertools::Itertools;
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    pyclass::CompareOp,
    types::{PyList, PyString},
};
use rustpython_parser::ast::{Expr, ExprContext, ExprKind, Operator, Stmt, StmtKind, Withitem};

#[pyclass(get_all, set_all)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    filename: String,
    start_line: i32,
    end_line: i32,
}

#[pymethods]
impl SourceSpan {
    #[new]
    fn new(filename: String, start_line: i32, end_line: i32) -> Self {
        Self {
            filename,
            start_line,
            end_line,
        }
    }

    fn __str__(&self) -> String {
        format!("{}:{}-{}", self.filename, self.start_line, self.end_line)
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
            _ => unimplemented!("only equality exists for SourceSpan"),
        }
    }
}

impl From<super::SourceSpan> for SourceSpan {
    fn from(value: super::SourceSpan) -> Self {
        Self {
            filename: value.path.to_str().unwrap().to_string(),
            start_line: value.start as i32,
            end_line: value.end as i32,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct ObjectPath {
    #[pyo3(get, set)]
    components: Vec<String>,

    formatted_path: String,
}

#[pymethods]
impl ObjectPath {
    fn append_part(&mut self, part: String) {
        self.components.push(part);
    }

    fn __str__(&self) -> String {
        self.components.join(".")
    }
}

impl From<super::ObjectPath> for ObjectPath {
    fn from(value: super::ObjectPath) -> Self {
        Self {
            formatted_path: value.to_string(),
            components: value.components,
        }
    }
}

#[pyclass(subclass, get_all, set_all)]
#[derive(Clone, Debug)]
pub struct Object {
    source_span: SourceSpan,
    object_path: ObjectPath,
    children: HashMap<String, PyObject>,
    name: String,
}

#[pymethods]
impl Object {
    #[new]
    fn new(source_span: SourceSpan, name: String, object_path: ObjectPath) -> Self {
        Self {
            source_span,
            object_path,
            name,
            children: HashMap::new(),
        }
    }

    fn __str__(&self) -> String {
        unimplemented!("Object is a base-class, no str representation")
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self == other),
            CompareOp::Ne => Ok(self != other),
            _ => unimplemented!("only equality exists for Object"),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.source_span == other.source_span && self.name == other.name
    }
}

impl Eq for Object {}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_span.hash(state);
        self.name.hash(state);
    }
}

#[pyclass(extends=Object, get_all, set_all)]
#[derive(Debug, Clone)]
pub struct AltObject {
    alt_name: String,
    sub_ob: Object,
}

#[pymethods]
impl AltObject {
    #[new]
    fn new(
        source_span: SourceSpan,
        name: String,
        object_path: ObjectPath,
        sub_ob: Object,
        alt_cnt: i32,
    ) -> (Self, Object) {
        let alt_name = format!("{}#{}", name, alt_cnt);
        let ob = Object::new(source_span, alt_name.clone(), object_path);
        let alt = AltObject { alt_name, sub_ob };
        (alt, ob)
    }
}

#[pyclass(extends=Object)]
#[derive(Clone, Debug)]
pub struct Module;

#[pymethods]
impl Module {
    #[new]
    fn new(source_span: SourceSpan, name: String, object_path: ObjectPath) -> (Self, Object) {
        (Self {}, Object::new(source_span, name, object_path))
    }

    fn __str__(&self) -> String {
        "module".into()
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

#[pyclass(extends=Object)]
#[derive(Clone, Debug)]
pub struct Class;

#[pymethods]
impl Class {
    #[new]
    fn new(source_span: SourceSpan, name: String, object_path: ObjectPath) -> (Self, Object) {
        (Self {}, Object::new(source_span, name, object_path))
    }

    fn __str__(&self) -> String {
        "class".into()
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

#[pyclass]
#[derive(Debug, Clone, Copy)]
pub enum FormalParamKind {
    POSONLY = 0,
    NORMAL = 1,
    KWONLY = 2,
}

impl From<super::FormalParamKind> for FormalParamKind {
    fn from(value: super::FormalParamKind) -> Self {
        match value {
            super::FormalParamKind::PosOnly => Self::POSONLY,
            super::FormalParamKind::KwOnly => Self::KWONLY,
            super::FormalParamKind::Normal => Self::NORMAL,
        }
    }
}

#[pyclass(get_all, set_all)]
#[derive(Debug, Clone)]
pub struct FormalParam {
    name: String,
    has_default: bool,
    kind: FormalParamKind,
}

#[pymethods]
impl FormalParam {
    #[new]
    fn new(name: String, has_default: bool, kind: FormalParamKind) -> Self {
        Self {
            name,
            has_default,
            kind,
        }
    }
}

impl From<super::FormalParam> for FormalParam {
    fn from(value: super::FormalParam) -> Self {
        Self {
            name: value.name,
            has_default: value.has_default,
            kind: value.kind.into(),
        }
    }
}

// FIXME: Add stmts
#[pyclass(extends=Object)]
#[derive(Clone, Debug)]
pub struct Function {
    formal_params: Vec<FormalParam>,
    kwarg: Option<String>,
    formatted_args: String,

    #[pyo3(get, set)]
    stmts: HashMap<i32, PyObject>,
}

#[pymethods]
impl Function {
    fn has_kwargs_dict(&self) -> bool {
        self.kwarg.is_some()
    }

    fn get_kwargs_name(&self) -> PyResult<String> {
        self.kwarg
            .clone()
            .ok_or_else(|| PyValueError::new_err("fn has not got keyword arguments"))
    }

    fn get_formal_params(&self) -> Vec<FormalParam> {
        self.formal_params.clone()
    }

    fn __repr__(self_: PyRef<'_, Self>) -> String {
        Function::__str__(self_)
    }

    fn __str__(self_: PyRef<'_, Self>) -> String {
        let super_ = self_.as_ref();
        format!(
            "function {}({})",
            super_.object_path.formatted_path, self_.formatted_args
        )
    }
}

pub type SymbolTable<'a> = HashMap<&'static str, &'a PyAny>;

fn get_ast_symbol_table(py: Python) -> PyResult<SymbolTable> {
    let symbols = [
        "Return",
        "Delete",
        "Assign",
        "AugAssign",
        "Load",
        "Store",
        "Del",
        "Name",
        "Add",
        "Sub",
        "Mult",
        "MatMult",
        "Div",
        "Mod",
        "Pow",
        "LShift",
        "RShift",
        "BitOr",
        "BitXor",
        "BitAnd",
        "FloorDiv",
        "AnnAssign",
        "For",
        "AsyncFor",
        "While",
        "If",
        "withitem",
        "With",
        "AsyncWith",
        "Pass",
        "Continue",
        "Break",
    ];

    let ast = PyModule::import(py, "ast")?;
    let mut table = SymbolTable::new();
    for symbol in symbols {
        let ob = ast.getattr(symbol)?;
        table.insert(symbol, ob);
    }
    Ok(table)
}

fn expr_ctx_to_py<'a>(ctx: ExprContext, ast: &SymbolTable<'a>) -> PyResult<&'a PyAny> {
    let class_name = match ctx {
        ExprContext::Load => "Load",
        ExprContext::Store => "Store",
        ExprContext::Del => "Del",
    };
    let class = ast[class_name];
    Ok(class.call0()?.downcast()?)
}

fn operator_to_py<'a>(op: Operator, ast: &SymbolTable<'a>) -> PyResult<&'a PyAny> {
    let class_name = match op {
        Operator::Add => "Add",
        Operator::Sub => "Sub",
        Operator::Mult => "Mult",
        Operator::MatMult => "MatMult",
        Operator::Div => "Div",
        Operator::Mod => "Mod",
        Operator::Pow => "Pow",
        Operator::LShift => "LShift",
        Operator::RShift => "RShift",
        Operator::BitOr => "BitOr",
        Operator::BitXor => "BitXor",
        Operator::BitAnd => "BitAnd",
        Operator::FloorDiv => "FloorDiv",
    };
    let class = ast[class_name];
    Ok(class.call0()?.downcast()?)
}

fn expr_kind_to_py<'a>(
    kind: ExprKind,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let none = py.None();

    let str_to_py = |s: &str| PyString::new(py, s);

    match kind {
        ExprKind::BoolOp { op, values } => todo!(),
        ExprKind::NamedExpr { target, value } => todo!(),
        ExprKind::BinOp { left, op, right } => todo!(),
        ExprKind::UnaryOp { op, operand } => todo!(),
        ExprKind::Lambda { args, body } => todo!(),
        ExprKind::IfExp { test, body, orelse } => todo!(),
        ExprKind::Dict { keys, values } => todo!(),
        ExprKind::Set { elts } => todo!(),
        ExprKind::ListComp { elt, generators } => todo!(),
        ExprKind::SetComp { elt, generators } => todo!(),
        ExprKind::DictComp {
            key,
            value,
            generators,
        } => todo!(),
        ExprKind::GeneratorExp { elt, generators } => todo!(),
        ExprKind::Await { value } => todo!(),
        ExprKind::Yield { value } => todo!(),
        ExprKind::YieldFrom { value } => todo!(),
        ExprKind::Compare {
            left,
            ops,
            comparators,
        } => todo!(),
        ExprKind::Call {
            func,
            args,
            keywords,
        } => todo!(),
        ExprKind::FormattedValue {
            value,
            conversion,
            format_spec,
        } => todo!(),
        ExprKind::JoinedStr { values } => todo!(),
        ExprKind::Constant { value, kind } => todo!(),
        ExprKind::Attribute { value, attr, ctx } => todo!(),
        ExprKind::Subscript { value, slice, ctx } => todo!(),
        ExprKind::Starred { value, ctx } => todo!(),
        ExprKind::Name { id, ctx } => {
            let name_class = ast["Name"];
            let id_py = str_to_py(&id);
            let ctx_py = expr_ctx_to_py(ctx, ast)?;
            let name_py = name_class.call1((id_py, ctx_py))?.downcast()?;
            Ok(name_py)
        }
        ExprKind::List { elts, ctx } => todo!(),
        ExprKind::Tuple { elts, ctx } => todo!(),
        ExprKind::Slice { lower, upper, step } => todo!(),
    }
}

fn with_item_to_py<'a>(
    with_item: Withitem,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let none = py.None();
    let context_expr_py = expr_kind_to_py(with_item.context_expr.node, py, ast)?;
    let opt_var_py = if let Some(opt_var) = with_item.optional_vars {
        expr_kind_to_py(opt_var.node, py, ast)?
    } else {
        none.as_ref(py)
    };
    let with_item_class = ast["withitem"];
    let with_item_var = with_item_class
        .call1((context_expr_py, opt_var_py))?
        .downcast()?;
    Ok(with_item_var)
}

fn stmt_kind_to_py<'a>(
    kind: StmtKind,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let none = py.None();

    let expr_vec_to_list = |exprs: Vec<Expr>| -> PyResult<&PyList> {
        Ok(PyList::new(
            py,
            exprs
                .into_iter()
                .map(|val| expr_kind_to_py(val.node, py, ast))
                .try_collect::<_, Vec<_>, _>()?
                .into_iter(),
        ))
    };
    let stmt_vec_to_list = |exprs: Vec<Stmt>| -> PyResult<&PyList> {
        Ok(PyList::new(
            py,
            exprs
                .into_iter()
                .map(|val| stmt_kind_to_py(val.node, py, ast))
                .try_collect::<_, Vec<_>, _>()?
                .into_iter(),
        ))
    };
    let opt_expr_to_py = |expr: Option<Box<Expr>>| -> PyResult<&PyAny> {
        if let Some(expr) = expr {
            expr_kind_to_py(expr.node, py, ast)
        } else {
            Ok(none.as_ref(py))
        }
    };
    let expr_to_py = |expr: Box<Expr>| expr_kind_to_py(expr.node, py, ast);
    let opt_str_to_py = |s: Option<String>| -> PyResult<&PyString> {
        if let Some(s) = s {
            Ok(PyString::new(py, &s))
        } else {
            Ok(none.downcast(py)?)
        }
    };

    match kind {
        StmtKind::FunctionDef { .. } => unreachable!("FunctionDef shouldn't exist in stmts"),
        StmtKind::AsyncFunctionDef { .. } => {
            unreachable!("AsyncFunctionDef shouldn't exist in stmts")
        }
        StmtKind::ClassDef { .. } => unreachable!("ClassDef shouldn't exist in stmts"),
        StmtKind::Return { value } => {
            let return_class = ast["Return"];
            let value_py = opt_expr_to_py(value)?;
            let return_val = return_class.call1((value_py,))?.downcast()?;
            Ok(return_val)
        }
        StmtKind::Delete { targets } => {
            let delete_class = ast["Delete"];
            let targets_py = expr_vec_to_list(targets)?;
            let delete_val = delete_class.call1((targets_py,))?.downcast()?;
            Ok(delete_val)
        }
        StmtKind::Assign {
            targets,
            value,
            type_comment,
        } => {
            let assign_class = ast["Assign"];
            let targets_py = expr_vec_to_list(targets)?;
            let value_py = expr_to_py(value)?;
            let type_comment_py = opt_str_to_py(type_comment)?;
            let assign_val = assign_class
                .call1((targets_py, value_py, type_comment_py))?
                .downcast()?;
            Ok(assign_val)
        }
        StmtKind::AugAssign { target, op, value } => {
            let aug_assign_class = ast["AugAssign"];
            let target_py = expr_to_py(target)?;
            let op_py = operator_to_py(op, ast)?;
            let value_py = expr_to_py(value)?;
            let aug_assign_val = aug_assign_class
                .call1((target_py, op_py, value_py))?
                .downcast()?;
            Ok(aug_assign_val)
        }
        StmtKind::AnnAssign {
            target,
            annotation,
            value,
            simple,
        } => {
            let ann_assign_class = ast["AnnAssign"];
            let target_py = expr_to_py(target)?;
            let annotation_py = expr_to_py(annotation)?;
            let value_py = opt_expr_to_py(value)?;
            let ann_assign_val = ann_assign_class
                .call1((target_py, annotation_py, value_py, simple))?
                .downcast()?;
            Ok(ann_assign_val)
        }
        StmtKind::For {
            target,
            iter,
            body,
            orelse,
            type_comment,
        } => {
            let for_class = ast["For"];
            let target_py = expr_to_py(target)?;
            let iter_py = expr_to_py(iter)?;
            let body_py = stmt_vec_to_list(body)?;
            let orelse_py = stmt_vec_to_list(orelse)?;
            let type_comment_py = opt_str_to_py(type_comment)?;
            let for_val = for_class
                .call1((target_py, iter_py, body_py, orelse_py, type_comment_py))?
                .downcast()?;
            Ok(for_val)
        }
        StmtKind::AsyncFor {
            target,
            iter,
            body,
            orelse,
            type_comment,
        } => {
            let async_for_class = ast["AsyncFor"];
            let target_py = expr_to_py(target)?;
            let iter_py = expr_to_py(iter)?;
            let body_py = stmt_vec_to_list(body)?;
            let orelse_py = stmt_vec_to_list(orelse)?;
            let type_comment_py = opt_str_to_py(type_comment)?;
            let async_for_val = async_for_class
                .call1((target_py, iter_py, body_py, orelse_py, type_comment_py))?
                .downcast()?;
            Ok(async_for_val)
        }
        StmtKind::While { test, body, orelse } => {
            let while_class = ast["While"];
            let test_py = expr_to_py(test)?;
            let body_py = stmt_vec_to_list(body)?;
            let orelse_py = stmt_vec_to_list(orelse)?;
            let while_val = while_class
                .call1((test_py, body_py, orelse_py))?
                .downcast()?;
            Ok(while_val)
        }
        StmtKind::If { test, body, orelse } => {
            let if_class = ast["If"];
            let test_py = expr_to_py(test)?;
            let body_py = stmt_vec_to_list(body)?;
            let orelse_py = stmt_vec_to_list(orelse)?;
            let if_val = if_class.call1((test_py, body_py, orelse_py))?.downcast()?;
            Ok(if_val)
        }
        StmtKind::With {
            items,
            body,
            type_comment,
        } => {
            let with_class = ast["With"];
            let items_py = PyList::new(
                py,
                items
                    .into_iter()
                    .map(|item| with_item_to_py(item, py, ast))
                    .try_collect::<_, Vec<_>, _>()?
                    .into_iter(),
            );
            let body_py = stmt_vec_to_list(body)?;
            let type_comment_py = opt_str_to_py(type_comment)?;
            let with_val = with_class
                .call1((items_py, body_py, type_comment_py))?
                .downcast()?;
            Ok(with_val)
        }
        StmtKind::AsyncWith {
            items,
            body,
            type_comment,
        } => {
            let async_with_class = ast["AsyncWith"];
            let items_py = PyList::new(
                py,
                items
                    .into_iter()
                    .map(|item| with_item_to_py(item, py, ast))
                    .try_collect::<_, Vec<_>, _>()?
                    .into_iter(),
            );
            let body_py = stmt_vec_to_list(body)?;
            let type_comment_py = opt_str_to_py(type_comment)?;
            let async_with_val = async_with_class
                .call1((items_py, body_py, type_comment_py))?
                .downcast()?;
            Ok(async_with_val)
        }
        StmtKind::Match { subject, cases } => todo!(),
        StmtKind::Raise { exc, cause } => todo!(),
        StmtKind::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => todo!(),
        StmtKind::Assert { test, msg } => todo!(),
        StmtKind::Import { names } => todo!(),
        StmtKind::ImportFrom {
            module,
            names,
            level,
        } => todo!(),
        StmtKind::Global { names } => todo!(),
        StmtKind::Nonlocal { names } => todo!(),
        StmtKind::Expr { value } => expr_to_py(value),
        StmtKind::Pass => Ok(ast["Pass"].call0()?.downcast()?),
        StmtKind::Break => Ok(ast["Break"].call0()?.downcast()?),
        StmtKind::Continue => Ok(ast["Continue"].call0()?.downcast()?),
    }
}

#[cfg(test)]
mod tests {
    use rustpython_parser::parser::parse_program;

    use super::*;

    fn parse_single_stmt(stmt: &str) -> StmtKind {
        let stmts = parse_program(stmt, "file.py").unwrap();
        stmts.into_iter().next().unwrap().node
    }

    #[test]
    fn test_stmt_kind_del() {
        pyo3::prepare_freethreaded_python();

        let del_stmt = parse_single_stmt("del a");

        Python::with_gil(|py| {
            let ast = get_ast_symbol_table(py).unwrap();
            let _ = stmt_kind_to_py(del_stmt, py, &ast).unwrap();
        });
    }
}
