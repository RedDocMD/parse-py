use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use itertools::Itertools;
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    pyclass::CompareOp,
    types::{PyComplex, PyTuple},
};
use rustpython_parser::ast::{
    Alias, Arg, Arguments, Boolop, Cmpop, Comprehension, Constant, Excepthandler,
    ExcepthandlerKind, Expr, ExprContext, ExprKind, KeywordData, MatchCase, Operator, PatternKind,
    Stmt, StmtKind, Unaryop, Withitem,
};

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

#[pyclass]
#[derive(Clone, Debug)]
pub struct ObjectPath {
    #[pyo3(get, set)]
    components: Vec<String>,

    formatted_path: String,
}

#[pymethods]
impl ObjectPath {
    #[new]
    fn new(components: Vec<String>, formatted_path: String) -> Self {
        Self {
            components,
            formatted_path,
        }
    }

    fn append_part(&mut self, part: String) {
        self.components.push(part);
    }

    fn __str__(&self) -> String {
        self.components.join(".")
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
    fn new(
        source_span: SourceSpan,
        name: String,
        object_path: ObjectPath,
        children: HashMap<String, PyObject>,
    ) -> Self {
        Self {
            source_span,
            object_path,
            name,
            children,
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
    sub_ob: PyObject,
}

#[pymethods]
impl AltObject {
    #[new]
    fn new(
        source_span: SourceSpan,
        name: String,
        object_path: ObjectPath,
        sub_ob: PyObject,
        children: HashMap<String, PyObject>,
    ) -> (Self, Object) {
        let ob = Object::new(source_span, name.clone(), object_path, children);
        let alt = AltObject {
            alt_name: name,
            sub_ob,
        };
        (alt, ob)
    }
}

#[pyclass(extends=Object)]
#[derive(Clone, Debug)]
pub struct Module;

#[pymethods]
impl Module {
    #[new]
    fn new(
        source_span: SourceSpan,
        name: String,
        object_path: ObjectPath,
        children: HashMap<String, PyObject>,
    ) -> (Self, Object) {
        (
            Self {},
            Object::new(source_span, name, object_path, children),
        )
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
    fn new(
        source_span: SourceSpan,
        name: String,
        object_path: ObjectPath,
        children: HashMap<String, PyObject>,
    ) -> (Self, Object) {
        (
            Self {},
            Object::new(source_span, name, object_path, children),
        )
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
    #[allow(clippy::too_many_arguments)]
    #[new]
    fn new(
        source_span: SourceSpan,
        name: String,
        object_path: ObjectPath,
        children: HashMap<String, PyObject>,
        formal_params: Vec<FormalParam>,
        formatted_args: String,
        stmts: HashMap<i32, PyObject>,
        kwarg: Option<String>,
    ) -> (Self, Object) {
        let func = Function {
            formal_params,
            kwarg,
            formatted_args,
            stmts,
        };
        let object = Object::new(source_span, name, object_path, children);
        (func, object)
    }

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
        "Expr",
        "MatchValue",
        "Constant",
        "MatchSingleton",
        "MatchSequence",
        "MatchMapping",
        "MatchClass",
        "MatchStar",
        "MatchAs",
        "MatchOr",
        "match_case",
        "Match",
        "Raise",
        "Try",
        "ExceptHandler",
        "Assert",
        "Import",
        "ImportFrom",
        "Global",
        "Nonlocal",
        "alias",
        "And",
        "Or",
        "BoolOp",
        "NamedExpr",
        "BinOp",
        "Invert",
        "Add",
        "UAdd",
        "USub",
        "UnaryOp",
        "IfExp",
        "Dict",
        "Set",
        "Await",
        "Yield",
        "YieldFrom",
        "Eq",
        "NotEq",
        "Lt",
        "LtE",
        "Gt",
        "GtE",
        "Is",
        "IsNot",
        "In",
        "NotIn",
        "Compare",
        "FormattedValue",
        "JoinedStr",
        "Constant",
        "Attribute",
        "Subscript",
        "Starred",
        "List",
        "Tuple",
        "Slice",
        "keyword",
        "Call",
        "comprehension",
        "ListComp",
        "SetComp",
        "GeneratorExp",
        "DictComp",
        "arg",
        "arguments",
        "Lambda",
    ];

    let ast = PyModule::import(py, "ast")?;
    let mut table = SymbolTable::new();
    for symbol in symbols {
        let ob = ast.getattr(symbol)?;
        table.insert(symbol, ob);
    }
    Ok(table)
}

#[rustfmt::skip]
macro_rules! py_value {
    ($ast:ident, $name:expr) => {
        Ok($ast[$name].call0()?.downcast()?)
    };
    ($ast:ident, $name:expr, $($arg:expr),+) => {
        Ok($ast[$name].call1(($($arg,)*))?.downcast()?)
    };
}

fn expr_ctx_to_py<'a>(ctx: ExprContext, ast: &SymbolTable<'a>) -> PyResult<&'a PyAny> {
    let class_name = match ctx {
        ExprContext::Load => "Load",
        ExprContext::Store => "Store",
        ExprContext::Del => "Del",
    };
    py_value!(ast, class_name)
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
    py_value!(ast, class_name)
}

fn bool_op_to_py<'a>(op: Boolop, ast: &SymbolTable<'a>) -> PyResult<&'a PyAny> {
    let class_name = match op {
        Boolop::And => "And",
        Boolop::Or => "Or",
    };
    py_value!(ast, class_name)
}

fn unary_op_to_py<'a>(op: Unaryop, ast: &SymbolTable<'a>) -> PyResult<&'a PyAny> {
    let class_name = match op {
        Unaryop::Invert => "Invert",
        Unaryop::Not => "Not",
        Unaryop::UAdd => "UAdd",
        Unaryop::USub => "USub",
    };
    py_value!(ast, class_name)
}

fn comp_op_to_py<'a>(op: Cmpop, ast: &SymbolTable<'a>) -> PyResult<&'a PyAny> {
    let class_name = match op {
        Cmpop::Eq => "Eq",
        Cmpop::NotEq => "NotEq",
        Cmpop::Lt => "Lt",
        Cmpop::LtE => "LtE",
        Cmpop::Gt => "Gt",
        Cmpop::GtE => "GtE",
        Cmpop::Is => "Is",
        Cmpop::IsNot => "IsNot",
        Cmpop::In => "In",
        Cmpop::NotIn => "NotIn",
    };
    py_value!(ast, class_name)
}

fn arg_to_py<'a>(arg: Arg, py: Python<'a>, ast: &SymbolTable<'a>) -> PyResult<&'a PyAny> {
    let annotation = arg
        .node
        .annotation
        .map(|e| expr_kind_to_py(e.node, py, ast))
        .transpose()?;
    py_value!(ast, "arg", arg.node.arg, annotation, arg.node.type_comment)
}

fn arguments_to_py<'a>(
    args: Arguments,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let args_to_py = |args: Vec<Arg>| -> PyResult<Vec<&PyAny>> {
        args.into_iter()
            .map(|a| arg_to_py(a, py, ast))
            .try_collect()
    };

    let opt_arg_to_py = |arg: Option<Box<Arg>>| arg.map(|a| arg_to_py(*a, py, ast)).transpose();

    let expr_vec_to_py = |exprs: Vec<Expr>| -> PyResult<Vec<_>> {
        exprs
            .into_iter()
            .map(|e| expr_kind_to_py(e.node, py, ast))
            .try_collect()
    };

    let posonlyargs = args_to_py(args.posonlyargs)?;
    let args_ = args_to_py(args.args)?;
    let kwonlyargs = args_to_py(args.kwonlyargs)?;
    let vararg = opt_arg_to_py(args.vararg)?;
    let kwarg = opt_arg_to_py(args.kwarg)?;
    let kw_defaults = expr_vec_to_py(args.kw_defaults)?;
    let defaults = expr_vec_to_py(args.defaults)?;

    py_value!(
        ast,
        "arguments",
        posonlyargs,
        args_,
        vararg,
        kwonlyargs,
        kw_defaults,
        kwarg,
        defaults
    )
}

fn expr_kind_to_py<'a>(
    kind: ExprKind,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let expr_to_py = |expr: Box<Expr>| expr_kind_to_py(expr.node, py, ast);

    let expr_vec_to_py = |exprs: Vec<Expr>| -> PyResult<Vec<_>> {
        exprs
            .into_iter()
            .map(|e| expr_kind_to_py(e.node, py, ast))
            .try_collect()
    };
    let opt_expr_to_py = |expr: Option<Box<Expr>>| expr.map(expr_to_py).transpose();

    let keyword_data_to_py = |data: KeywordData| -> PyResult<&PyAny> {
        let value = expr_kind_to_py(data.value.node, py, ast)?;
        py_value!(ast, "keyword", data.arg, value)
    };

    let comprehension_to_py = |comprehension: Comprehension| -> PyResult<&PyAny> {
        let target = expr_kind_to_py(comprehension.target.node, py, ast)?;
        let iter = expr_kind_to_py(comprehension.iter.node, py, ast)?;
        let ifs = expr_vec_to_py(comprehension.ifs)?;
        py_value!(
            ast,
            "comprehension",
            target,
            iter,
            ifs,
            comprehension.is_async
        )
    };

    let comprehension_vec_to_py = |cprs: Vec<Comprehension>| -> PyResult<Vec<&PyAny>> {
        cprs.into_iter().map(comprehension_to_py).try_collect()
    };

    match kind {
        ExprKind::BoolOp { op, values } => {
            let op = bool_op_to_py(op, ast)?;
            let values = expr_vec_to_py(values)?;
            py_value!(ast, "BoolOp", op, values)
        }
        ExprKind::NamedExpr { target, value } => {
            let target = expr_to_py(target)?;
            let value = expr_to_py(value)?;
            py_value!(ast, "NamedExpr", target, value)
        }
        ExprKind::BinOp { left, op, right } => {
            let left = expr_to_py(left)?;
            let op = operator_to_py(op, ast)?;
            let right = expr_to_py(right)?;
            py_value!(ast, "BinOp", left, op, right)
        }
        ExprKind::UnaryOp { op, operand } => {
            let op = unary_op_to_py(op, ast)?;
            let operand = expr_to_py(operand)?;
            py_value!(ast, "UnaryOp", op, operand)
        }
        ExprKind::Lambda { args, body } => {
            let args = arguments_to_py(*args, py, ast)?;
            let body = expr_to_py(body)?;
            py_value!(ast, "Lambda", args, body)
        }
        ExprKind::IfExp { test, body, orelse } => {
            let test = expr_to_py(test)?;
            let body = expr_to_py(body)?;
            let orelse = expr_to_py(orelse)?;
            py_value!(ast, "IfExp", test, body, orelse)
        }
        ExprKind::Dict { keys, values } => {
            let keys = expr_vec_to_py(keys)?;
            let values = expr_vec_to_py(values)?;
            py_value!(ast, "Dict", keys, values)
        }
        ExprKind::Set { elts } => {
            let elts = expr_vec_to_py(elts)?;
            py_value!(ast, "Set", elts)
        }
        ExprKind::ListComp { elt, generators } => {
            let elt = expr_to_py(elt)?;
            let generators = comprehension_vec_to_py(generators)?;
            py_value!(ast, "ListComp", elt, generators)
        }
        ExprKind::SetComp { elt, generators } => {
            let elt = expr_to_py(elt)?;
            let generators = comprehension_vec_to_py(generators)?;
            py_value!(ast, "SetComp", elt, generators)
        }
        ExprKind::DictComp {
            key,
            value,
            generators,
        } => {
            let key = expr_to_py(key)?;
            let value = expr_to_py(value)?;
            let generators = comprehension_vec_to_py(generators)?;
            py_value!(ast, "DictComp", key, value, generators)
        }
        ExprKind::GeneratorExp { elt, generators } => {
            let elt = expr_to_py(elt)?;
            let generators = comprehension_vec_to_py(generators)?;
            py_value!(ast, "GeneratorExp", elt, generators)
        }
        ExprKind::Await { value } => {
            let value = expr_to_py(value)?;
            py_value!(ast, "Await", value)
        }
        ExprKind::Yield { value } => {
            let value = opt_expr_to_py(value)?;
            py_value!(ast, "Yield", value)
        }
        ExprKind::YieldFrom { value } => {
            let value = expr_to_py(value)?;
            py_value!(ast, "YieldFrom", value)
        }
        ExprKind::Compare {
            left,
            ops,
            comparators,
        } => {
            let left = expr_to_py(left)?;
            let ops: Vec<_> = ops
                .into_iter()
                .map(|op| comp_op_to_py(op, ast))
                .try_collect()?;
            let comparators = expr_vec_to_py(comparators)?;
            py_value!(ast, "Compare", left, ops, comparators)
        }
        ExprKind::Call {
            func,
            args,
            keywords,
        } => {
            let func = expr_to_py(func)?;
            let args = expr_vec_to_py(args)?;
            let keywords: Vec<_> = keywords
                .into_iter()
                .map(|k| keyword_data_to_py(k.node))
                .try_collect()?;
            py_value!(ast, "Call", func, args, keywords)
        }
        ExprKind::FormattedValue {
            value,
            conversion,
            format_spec,
        } => {
            let value = expr_to_py(value)?;
            let format_spec = opt_expr_to_py(format_spec)?;
            py_value!(ast, "FormattedValue", value, conversion, format_spec)
        }
        ExprKind::JoinedStr { values } => {
            let values = expr_vec_to_py(values)?;
            py_value!(ast, "JoinedStr", values)
        }
        ExprKind::Constant { value, kind } => {
            let value = constant_to_py(value, py, ast)?;
            py_value!(ast, "Constant", value, kind)
        }
        ExprKind::Attribute { value, attr, ctx } => {
            let value = expr_to_py(value)?;
            let ctx = expr_ctx_to_py(ctx, ast)?;
            py_value!(ast, "Attribute", value, attr, ctx)
        }
        ExprKind::Subscript { value, slice, ctx } => {
            let value = expr_to_py(value)?;
            let slice = expr_to_py(slice)?;
            let ctx = expr_ctx_to_py(ctx, ast)?;
            py_value!(ast, "Subscript", value, slice, ctx)
        }
        ExprKind::Starred { value, ctx } => {
            let value = expr_to_py(value)?;
            let ctx = expr_ctx_to_py(ctx, ast)?;
            py_value!(ast, "Starred", value, ctx)
        }
        ExprKind::Name { id, ctx } => {
            let ctx = expr_ctx_to_py(ctx, ast)?;
            py_value!(ast, "Name", id, ctx)
        }
        ExprKind::List { elts, ctx } => {
            let elts = expr_vec_to_py(elts)?;
            let ctx = expr_ctx_to_py(ctx, ast)?;
            py_value!(ast, "List", elts, ctx)
        }
        ExprKind::Tuple { elts, ctx } => {
            let elts = expr_vec_to_py(elts)?;
            let ctx = expr_ctx_to_py(ctx, ast)?;
            py_value!(ast, "Tuple", elts, ctx)
        }
        ExprKind::Slice { lower, upper, step } => {
            let lower = opt_expr_to_py(lower)?;
            let upper = opt_expr_to_py(upper)?;
            let step = opt_expr_to_py(step)?;
            py_value!(ast, "Slice", lower, upper, step)
        }
    }
}

fn with_item_to_py<'a>(
    with_item: Withitem,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let context_expr = expr_kind_to_py(with_item.context_expr.node, py, ast)?;
    let opt_var = with_item
        .optional_vars
        .map(|e| expr_kind_to_py(e.node, py, ast))
        .transpose()?;
    py_value!(ast, "withitem", context_expr, opt_var)
}

fn constant_to_py<'a>(
    kind: Constant,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let none = py.None();
    let ellipsis = py.Ellipsis();

    let value = match kind {
        Constant::None => none,
        Constant::Bool(b) => b.into_py(py),
        Constant::Str(s) => s.into_py(py),
        Constant::Bytes(b) => b.into_py(py),
        // FIXME: Handle BigInt properly
        Constant::Int(_i) => 1.into_py(py),
        Constant::Tuple(t) => PyTuple::new(
            py,
            t.into_iter()
                .map(|c| constant_to_py(c, py, ast))
                .try_collect::<_, Vec<_>, _>()?,
        )
        .into_py(py),
        Constant::Float(f) => f.into_py(py),
        Constant::Complex { real, imag } => PyComplex::from_doubles(py, real, imag).into_py(py),
        Constant::Ellipsis => ellipsis,
    };

    py_value!(ast, "Constant", value)
}

fn match_pattern_to_py<'a>(
    kind: PatternKind,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let expr_to_py = |expr: Box<Expr>| expr_kind_to_py(expr.node, py, ast);

    match kind {
        PatternKind::MatchValue { value } => {
            let value = expr_to_py(value)?;
            py_value!(ast, "MatchValue", value)
        }
        PatternKind::MatchSingleton { value } => {
            let value = constant_to_py(value, py, ast)?;
            py_value!(ast, "MatchSingleton", value)
        }
        PatternKind::MatchSequence { patterns } => {
            let patterns: Vec<_> = patterns
                .into_iter()
                .map(|c| match_pattern_to_py(c.node, py, ast))
                .try_collect()?;
            py_value!(ast, "MatchSequence", patterns)
        }
        PatternKind::MatchMapping {
            keys,
            patterns,
            rest,
        } => {
            let keys: Vec<_> = keys
                .into_iter()
                .map(|c| expr_kind_to_py(c.node, py, ast))
                .try_collect()?;
            let patterns: Vec<_> = patterns
                .into_iter()
                .map(|c| match_pattern_to_py(c.node, py, ast))
                .try_collect()?;
            py_value!(ast, "MatchMapping", keys, patterns, rest)
        }
        PatternKind::MatchClass {
            cls,
            patterns,
            kwd_attrs,
            kwd_patterns,
        } => {
            let cls = expr_to_py(cls)?;
            let patterns: Vec<_> = patterns
                .into_iter()
                .map(|c| match_pattern_to_py(c.node, py, ast))
                .try_collect()?;
            let kwd_patterns: Vec<_> = kwd_patterns
                .into_iter()
                .map(|c| match_pattern_to_py(c.node, py, ast))
                .try_collect()?;
            py_value!(ast, "MatchClass", cls, patterns, kwd_attrs, kwd_patterns)
        }
        PatternKind::MatchStar { name } => py_value!(ast, "MatchStar", name),
        PatternKind::MatchAs { pattern, name } => {
            let pattern = pattern
                .map(|p| match_pattern_to_py(p.node, py, ast))
                .transpose()?;
            py_value!(ast, "MatchAs", pattern, name)
        }
        PatternKind::MatchOr { patterns } => {
            let patterns: Vec<_> = patterns
                .into_iter()
                .map(|c| match_pattern_to_py(c.node, py, ast))
                .try_collect()?;
            py_value!(ast, "MatchOr", patterns)
        }
    }
}

fn match_case_to_py<'a>(
    mc: MatchCase,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let pattern = match_pattern_to_py(mc.pattern.node, py, ast)?;
    let guard = mc
        .guard
        .map(|e| expr_kind_to_py(e.node, py, ast))
        .transpose()?;
    let body: Vec<_> = mc
        .body
        .into_iter()
        .map(|val| stmt_kind_to_py(val.node, py, ast))
        .try_collect()?;
    py_value!(ast, "match_case", pattern, guard, body)
}

fn stmt_kind_to_py<'a>(
    kind: StmtKind,
    py: Python<'a>,
    ast: &SymbolTable<'a>,
) -> PyResult<&'a PyAny> {
    let expr_vec_to_list = |exprs: Vec<Expr>| -> PyResult<Vec<&PyAny>> {
        exprs
            .into_iter()
            .map(|val| expr_kind_to_py(val.node, py, ast))
            .try_collect()
    };
    let stmt_vec_to_list = |stmts: Vec<Stmt>| -> PyResult<Vec<&PyAny>> {
        stmts
            .into_iter()
            .map(|val| stmt_kind_to_py(val.node, py, ast))
            .try_collect()
    };
    let expr_to_py = |expr: Box<Expr>| expr_kind_to_py(expr.node, py, ast);
    let opt_expr_to_py = |expr: Option<Box<Expr>>| expr.map(expr_to_py).transpose();
    let except_to_py = |e: Excepthandler| -> PyResult<&PyAny> {
        match e.node {
            ExcepthandlerKind::ExceptHandler { type_, name, body } => {
                let type_ = opt_expr_to_py(type_)?;
                let body = stmt_vec_to_list(body)?;
                py_value!(ast, "ExceptHandler", type_, name, body)
            }
        }
    };
    let alias_to_py =
        |a: Alias| -> PyResult<&PyAny> { py_value!(ast, "alias", a.node.name, a.node.asname) };

    match kind {
        StmtKind::FunctionDef { .. } => unreachable!("FunctionDef shouldn't exist in stmts"),
        StmtKind::AsyncFunctionDef { .. } => {
            unreachable!("AsyncFunctionDef shouldn't exist in stmts")
        }
        StmtKind::ClassDef { .. } => unreachable!("ClassDef shouldn't exist in stmts"),
        StmtKind::Return { value } => {
            let value = opt_expr_to_py(value)?;
            py_value!(ast, "Return", value)
        }
        StmtKind::Delete { targets } => {
            let targets = expr_vec_to_list(targets)?;
            py_value!(ast, "Delete", targets)
        }
        StmtKind::Assign {
            targets,
            value,
            type_comment,
        } => {
            let targets = expr_vec_to_list(targets)?;
            let value = expr_to_py(value)?;
            py_value!(ast, "Assign", targets, value, type_comment)
        }
        StmtKind::AugAssign { target, op, value } => {
            let target = expr_to_py(target)?;
            let op = operator_to_py(op, ast)?;
            let value = expr_to_py(value)?;
            py_value!(ast, "AugAssign", target, op, value)
        }
        StmtKind::AnnAssign {
            target,
            annotation,
            value,
            simple,
        } => {
            let target = expr_to_py(target)?;
            let annotation = expr_to_py(annotation)?;
            let value = opt_expr_to_py(value)?;
            py_value!(ast, "AnnAssign", target, annotation, value, simple)
        }
        StmtKind::For {
            target,
            iter,
            body,
            orelse,
            type_comment,
        } => {
            let target = expr_to_py(target)?;
            let iter = expr_to_py(iter)?;
            let body = stmt_vec_to_list(body)?;
            let orelse = stmt_vec_to_list(orelse)?;
            py_value!(ast, "For", target, iter, body, orelse, type_comment)
        }
        StmtKind::AsyncFor {
            target,
            iter,
            body,
            orelse,
            type_comment,
        } => {
            let target = expr_to_py(target)?;
            let iter = expr_to_py(iter)?;
            let body = stmt_vec_to_list(body)?;
            let orelse = stmt_vec_to_list(orelse)?;
            py_value!(ast, "AsyncFor", target, iter, body, orelse, type_comment)
        }
        StmtKind::While { test, body, orelse } => {
            let test = expr_to_py(test)?;
            let body = stmt_vec_to_list(body)?;
            let orelse = stmt_vec_to_list(orelse)?;
            py_value!(ast, "While", test, body, orelse)
        }
        StmtKind::If { test, body, orelse } => {
            let test = expr_to_py(test)?;
            let body = stmt_vec_to_list(body)?;
            let orelse = stmt_vec_to_list(orelse)?;
            py_value!(ast, "If", test, body, orelse)
        }
        StmtKind::With {
            items,
            body,
            type_comment,
        } => {
            let items: Vec<_> = items
                .into_iter()
                .map(|item| with_item_to_py(item, py, ast))
                .try_collect()?;
            let body = stmt_vec_to_list(body)?;
            py_value!(ast, "With", items, body, type_comment)
        }
        StmtKind::AsyncWith {
            items,
            body,
            type_comment,
        } => {
            let items: Vec<_> = items
                .into_iter()
                .map(|item| with_item_to_py(item, py, ast))
                .try_collect()?;
            let body = stmt_vec_to_list(body)?;
            py_value!(ast, "AsyncWith", items, body, type_comment)
        }
        StmtKind::Match { subject, cases } => {
            let subject = expr_to_py(subject)?;
            let cases: Vec<_> = cases
                .into_iter()
                .map(|c| match_case_to_py(c, py, ast))
                .try_collect()?;
            py_value!(ast, "Match", subject, cases)
        }
        StmtKind::Raise { exc, cause } => {
            let exc = opt_expr_to_py(exc)?;
            let cause = opt_expr_to_py(cause)?;
            py_value!(ast, "Raise", exc, cause)
        }
        StmtKind::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            let body = stmt_vec_to_list(body)?;
            let handlers: Vec<_> = handlers.into_iter().map(except_to_py).try_collect()?;
            let orelse = stmt_vec_to_list(orelse)?;
            let finalbody = stmt_vec_to_list(finalbody)?;
            py_value!(ast, "Try", body, handlers, orelse, finalbody)
        }
        StmtKind::Assert { test, msg } => {
            let test = expr_to_py(test)?;
            let msg = opt_expr_to_py(msg)?;
            py_value!(ast, "Assert", test, msg)
        }
        StmtKind::Import { names } => {
            let names: Vec<_> = names.into_iter().map(alias_to_py).try_collect()?;
            py_value!(ast, "Import", names)
        }
        StmtKind::ImportFrom {
            module,
            names,
            level,
        } => {
            let names: Vec<_> = names.into_iter().map(alias_to_py).try_collect()?;
            py_value!(ast, "ImportFrom", module, names, level)
        }
        StmtKind::Global { names } => py_value!(ast, "Global", names),
        StmtKind::Nonlocal { names } => py_value!(ast, "Nonlocal", names),
        StmtKind::Expr { value } => {
            let value = expr_to_py(value)?;
            py_value!(ast, "Expr", value)
        }
        StmtKind::Pass => py_value!(ast, "Pass"),
        StmtKind::Break => py_value!(ast, "Break"),
        StmtKind::Continue => py_value!(ast, "Continue"),
    }
}

fn source_span_to_py(py: Python, span: super::SourceSpan) -> PyResult<&PyAny> {
    let span_type = py.get_type::<SourceSpan>();
    let val = span_type
        .call1((
            span.path.to_str().unwrap().to_string(),
            span.start,
            span.end,
        ))?
        .downcast()?;
    Ok(val)
}

fn object_path_to_py(py: Python, path: super::ObjectPath) -> PyResult<&PyAny> {
    let path_type = py.get_type::<ObjectPath>();
    let formatted_args = path.to_string();
    let val = path_type
        .call1((path.components, formatted_args))?
        .downcast()?;
    Ok(val)
}

pub fn module_to_py(py: Python, module: super::Module) -> PyResult<&PyAny> {
    let mod_type = py.get_type::<Module>();
    let name = module.name().to_string();
    let ss = source_span_to_py(py, module.data.span)?;
    let path = object_path_to_py(py, module.data.obj_path)?;
    let children: HashMap<_, _> = module
        .data
        .children
        .into_iter()
        .map(|(k, v)| object_to_py(py, v).map(|v| (k, v.into_py(py))))
        .try_collect()?;
    let val = mod_type.call1((ss, name, path, children))?.downcast()?;
    Ok(val)
}

fn class_to_py(py: Python, class: super::Class) -> PyResult<&PyAny> {
    let class_type = py.get_type::<Class>();
    let name = class.data.name().to_string();
    let ss = source_span_to_py(py, class.data.span)?;
    let path = object_path_to_py(py, class.data.obj_path)?;
    let children: HashMap<_, _> = class
        .data
        .children
        .into_iter()
        .map(|(k, v)| object_to_py(py, v).map(|v| (k, v.into_py(py))))
        .try_collect()?;
    let val = class_type.call1((ss, name, path, children))?.downcast()?;
    Ok(val)
}

fn formal_param_to_py(py: Python, fp: super::FormalParam) -> PyResult<&PyAny> {
    let kind = match fp.kind {
        super::FormalParamKind::PosOnly => FormalParamKind::POSONLY,
        super::FormalParamKind::KwOnly => FormalParamKind::KWONLY,
        super::FormalParamKind::Normal => FormalParamKind::NORMAL,
    }
    .into_py(py);
    let fp_type = py.get_type::<FormalParam>();
    let val = fp_type.call1((fp.name, fp.has_default, kind))?.downcast()?;
    Ok(val)
}

fn function_to_py(py: Python, func: super::Function) -> PyResult<&PyAny> {
    let func_type = py.get_type::<Function>();
    let data = func.data.clone();
    let name = data.name().to_string();
    let ss = source_span_to_py(py, data.span)?;
    let path = object_path_to_py(py, data.obj_path)?;
    let children: HashMap<_, _> = data
        .children
        .into_iter()
        .map(|(k, v)| object_to_py(py, v).map(|v| (k, v.into_py(py))))
        .try_collect()?;
    let formal_params: Vec<_> = func
        .formal_params()
        .into_iter()
        .map(|fp| formal_param_to_py(py, fp))
        .try_collect()?;
    let kwarg = if func.has_kwargs_dict() {
        Some(func.kwargs_name())
    } else {
        None
    };
    let formatted_args = func.format_args();
    let ast = get_ast_symbol_table(py)?;
    let stmts: HashMap<_, _> = func
        .stmts
        .into_iter()
        .map(|(k, v)| stmt_kind_to_py(v, py, &ast).map(|v| (k as i32, v.into_py(py))))
        .try_collect()?;
    let val = func_type
        .call1((
            ss,
            name,
            path,
            children,
            formal_params,
            formatted_args,
            stmts,
            kwarg,
        ))?
        .downcast()?;
    Ok(val)
}

fn alt_object_to_py(py: Python, alt_ob: super::AltObject) -> PyResult<&PyAny> {
    let alt_object_type = py.get_type::<AltObject>();
    let name = alt_ob.data.name().to_string();
    let ss = source_span_to_py(py, alt_ob.data.span)?;
    let path = object_path_to_py(py, alt_ob.data.obj_path)?;
    let sub_ob = object_to_py(py, *alt_ob.sub_ob)?;
    let children: HashMap<_, _> = alt_ob
        .data
        .children
        .into_iter()
        .map(|(k, v)| object_to_py(py, v).map(|v| (k, v.into_py(py))))
        .try_collect()?;
    let val = alt_object_type
        .call1((ss, name, path, sub_ob, children))?
        .downcast()?;
    Ok(val)
}

fn object_to_py(py: Python, ob: super::Object) -> PyResult<&PyAny> {
    match ob {
        super::Object::Module(module) => module_to_py(py, module),
        super::Object::Class(class) => class_to_py(py, class),
        super::Object::Function(func) => function_to_py(py, func),
        super::Object::AltObject(alt_ob) => alt_object_to_py(py, alt_ob),
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

    #[test]
    fn test_stmt_kind_for() {
        pyo3::prepare_freethreaded_python();

        let for_stmt = parse_single_stmt(
            "
for a in b:
    a + c
",
        );

        Python::with_gil(|py| {
            let ast = get_ast_symbol_table(py).unwrap();
            let _ = stmt_kind_to_py(for_stmt, py, &ast).unwrap();
        });
    }
}
