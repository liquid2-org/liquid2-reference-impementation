//! Liquid template syntax tree
//!
use pyo3::prelude::*;
use std::fmt::{self};

use crate::query::Query;

#[pyclass]
#[derive(Debug, Clone)]
pub struct Template {
    #[pyo3(get)]
    pub liquid: Vec<Node>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum Node {
    EOI {},
    Content {
        text: String,
    },
    Output {
        whitespace_control: WhitespaceControl,
        expression: FilteredExpression,
    },
    Raw {
        whitespace_control: (WhitespaceControl, WhitespaceControl),
        text: String,
    },
    Comment {
        whitespace_control: WhitespaceControl,
        text: String,
    },
    AssignTag {
        whitespace_control: WhitespaceControl,
        identifier: String,
        expression: FilteredExpression,
    },
    CaptureTag {
        whitespace_control: (WhitespaceControl, WhitespaceControl),
        identifier: String,
        block: Vec<Node>,
    },
    CaseTag {
        whitespace_control: (WhitespaceControl, WhitespaceControl),
        arg: Primitive,
        whens: Vec<WhenTag>,
        default: Option<ElseTag>,
    },
    CycleTag {
        whitespace_control: WhitespaceControl,
        name: Option<String>,
        args: Vec<Primitive>,
    },
    DecrementTag {
        whitespace_control: WhitespaceControl,
        name: String,
    },
    IncrementTag {
        whitespace_control: WhitespaceControl,
        name: String,
    },
    EchoTag {
        whitespace_control: WhitespaceControl,
        expression: FilteredExpression,
    },
    ForTag {
        whitespace_control: (WhitespaceControl, WhitespaceControl),
        name: String,
        iterable: Primitive,
        limit: Option<Primitive>,
        offset: Option<Primitive>,
        reversed: bool,
        block: Vec<Node>,
        default: Option<ElseTag>,
    },
    BreakTag {
        whitespace_control: WhitespaceControl,
    },
    ContinueTag {
        whitespace_control: WhitespaceControl,
    },
    IfTag {
        whitespace_control: (WhitespaceControl, WhitespaceControl),
        condition: BooleanExpression,
        block: Vec<Node>,
        alternatives: Vec<ElsifTag>,
        default: Option<ElseTag>,
    },
    UnlessTag {
        whitespace_control: (WhitespaceControl, WhitespaceControl),
        condition: BooleanExpression,
        block: Vec<Node>,
        alternatives: Vec<ElsifTag>,
        default: Option<ElseTag>,
    },
    IncludeTag {
        whitespace_control: WhitespaceControl,
        target: Primitive,
        repeat: bool,
        variable: Option<Primitive>,
        alias: Option<String>,
        args: Option<Vec<CommonArgument>>,
    },
    RenderTag {
        whitespace_control: WhitespaceControl,
        target: Primitive,
        repeat: bool,
        variable: Option<Primitive>,
        alias: Option<String>,
        args: Option<Vec<CommonArgument>>,
    },
    LiquidTag {
        whitespace_control: WhitespaceControl,
        block: Vec<Node>,
    },
    TagExtension {
        whitespace_control: (WhitespaceControl, Option<WhitespaceControl>),
        name: String,
        args: Vec<CommonArgument>,
        block: Option<Vec<Node>>,
        tags: Option<Vec<Node>>, // Nested tags, like `else` in a `for` loop, or `when` in a `case` block
    },
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct FilteredExpression {
    #[pyo3(get)]
    pub left: Primitive,
    #[pyo3(get)]
    pub filters: Option<Vec<Filter>>,
    #[pyo3(get)]
    pub condition: Option<InlineCondition>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct InlineCondition {
    #[pyo3(get)]
    pub expr: BooleanExpression,
    #[pyo3(get)]
    pub alternative: Option<Primitive>,
    #[pyo3(get)]
    pub alternative_filters: Option<Vec<Filter>>,
    #[pyo3(get)]
    pub tail_filters: Option<Vec<Filter>>,
}

impl fmt::Display for InlineCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "if {}", self.expr)?;

        self.alternative
            .as_ref()
            .and_then(|alt| Some(write!(f, " else {alt}")));

        self.alternative_filters.as_ref().and_then(|filters| {
            Some(write!(
                f,
                " | {}",
                filters
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(" | ")
            ))
        });

        self.tail_filters.as_ref().and_then(|filters| {
            Some(write!(
                f,
                " || {}",
                filters
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(" | ")
            ))
        });

        Ok(())
    }
}

#[pymethods]
impl InlineCondition {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum BooleanExpression {
    Primitive {
        expr: Primitive,
    },
    LogicalNot {
        expr: Box<BooleanExpression>,
    },
    Logical {
        left: Box<BooleanExpression>,
        operator: BooleanOperator,
        right: Box<BooleanExpression>,
    },
    Comparison {
        left: Primitive,
        operator: CompareOperator,
        right: Primitive,
    },
    Membership {
        left: Primitive,
        operator: MembershipOperator,
        right: Primitive,
    },
}

impl fmt::Display for BooleanExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BooleanExpression::Primitive { expr } => write!(f, "{expr}"),
            BooleanExpression::LogicalNot { expr } => write!(f, "not ({expr})"),
            BooleanExpression::Logical {
                left,
                operator,
                right,
            } => write!(f, "{left} {operator}, {right}"),
            BooleanExpression::Comparison {
                left,
                operator,
                right,
            } => write!(f, "{left} {operator}, {right}"),
            BooleanExpression::Membership {
                left,
                operator,
                right,
            } => write!(f, "{left} {operator}, {right}"),
        }
    }
}

#[pymethods]
impl BooleanExpression {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum BooleanOperator {
    And {},
    Or {},
}

impl fmt::Display for BooleanOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BooleanOperator::And {} => f.write_str("and"),
            BooleanOperator::Or {} => f.write_str("or"),
        }
    }
}

#[pymethods]
impl BooleanOperator {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum CompareOperator {
    Eq {},
    Ne {},
    Ge {},
    Gt {},
    Le {},
    Lt {},
}

impl fmt::Display for CompareOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOperator::Eq {} => f.write_str("=="),
            CompareOperator::Ne {} => f.write_str("!="),
            CompareOperator::Ge {} => f.write_str(">="),
            CompareOperator::Gt {} => f.write_str(">"),
            CompareOperator::Le {} => f.write_str("<="),
            &CompareOperator::Lt {} => f.write_str("<"),
        }
    }
}

#[pymethods]
impl CompareOperator {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum MembershipOperator {
    In {},
    NotIn {},
    Contains {},
    NotContains {},
}

impl fmt::Display for MembershipOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MembershipOperator::In {} => f.write_str("in"),
            MembershipOperator::NotIn {} => f.write_str("not in"),
            MembershipOperator::Contains {} => f.write_str("contains"),
            &MembershipOperator::NotContains {} => f.write_str("not contains"),
        }
    }
}

#[pymethods]
impl MembershipOperator {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Filter {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub args: Option<Vec<CommonArgument>>,
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Filter {
                name,
                args: Some(arguments),
            } => {
                write!(
                    f,
                    "{}: {}",
                    name,
                    arguments
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                )
            }
            Filter { name, args: None } => write!(f, "{name}"),
        }
    }
}

#[pymethods]
impl Filter {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum Primitive {
    TrueLiteral {},
    FalseLiteral {},
    NullLiteral {},
    Integer { value: i64 },
    Float { value: f64 },
    StringLiteral { value: String },
    Range { start: i64, stop: i64 },
    Query { path: Query },
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::TrueLiteral {} => f.write_str("true"),
            Primitive::FalseLiteral {} => f.write_str("false"),
            Primitive::NullLiteral {} => f.write_str("null"),
            Primitive::Integer { value } => write!(f, "{value}"),
            Primitive::Float { value } => write!(f, "{value}"),
            Primitive::StringLiteral { value } => write!(f, "\"{value}\""),
            Primitive::Range { start, stop } => write!(f, "({start}..{stop})"),
            Primitive::Query { path } => write!(f, "{path}"),
        }
    }
}

#[pymethods]
impl Primitive {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct WhenTag {
    #[pyo3(get)]
    pub whitespace_control: WhitespaceControl,
    #[pyo3(get)]
    pub args: Vec<Primitive>,
    #[pyo3(get)]
    pub block: Vec<Node>,
}

impl fmt::Display for WhenTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{%{} when {} {}%}}{}",
            self.whitespace_control.left,
            self.args
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            self.whitespace_control.right,
            display_block(&self.block)
        )
    }
}

#[pymethods]
impl WhenTag {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ElseTag {
    #[pyo3(get)]
    pub whitespace_control: WhitespaceControl,
    #[pyo3(get)]
    pub block: Vec<Node>,
}

impl fmt::Display for ElseTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{%{} else {}%}}{}",
            self.whitespace_control.left,
            self.whitespace_control.right,
            display_block(&self.block)
        )
    }
}

#[pymethods]
impl ElseTag {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ElsifTag {
    #[pyo3(get)]
    pub whitespace_control: WhitespaceControl,
    #[pyo3(get)]
    pub condition: BooleanExpression,
    #[pyo3(get)]
    pub block: Vec<Node>,
}

impl fmt::Display for ElsifTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{%{} elsif {} {}%}}{}",
            self.whitespace_control.left,
            self.condition,
            self.whitespace_control.right,
            display_block(&self.block)
        )
    }
}

#[pymethods]
impl ElsifTag {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct CommonArgument {
    #[pyo3(get)]
    pub value: Option<Primitive>,
    #[pyo3(get)]
    pub name: Option<String>,
}

impl fmt::Display for CommonArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommonArgument {
                value: Some(v),
                name: Some(k),
            } => write!(f, "{}:{}", k, v),
            CommonArgument {
                value: Some(v),
                name: None,
            } => write!(f, "{}", v),
            CommonArgument {
                value: None,
                name: Some(k),
            } => write!(f, "{}", k),
            _ => Ok(()),
        }
    }
}

#[pymethods]
impl CommonArgument {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct WhitespaceControl {
    #[pyo3(get)]
    pub left: Whitespace,
    #[pyo3(get)]
    pub right: Whitespace,
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq)]
pub enum Whitespace {
    Plus,
    Minus,
    Smart,
    Default,
}

impl Whitespace {
    pub fn from_str(s: &str) -> Self {
        match s {
            "+" => Self::Plus,
            "-" => Self::Minus,
            "~" => Self::Smart,
            "" => Self::Default,
            _ => unreachable!("{:#?}", s),
        }
    }
}

impl fmt::Display for Whitespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Whitespace::Plus => write!(f, "+"),
            Whitespace::Minus => write!(f, "-"),
            Whitespace::Smart => write!(f, "~"),
            Whitespace::Default => Ok(()),
        }
    }
}

#[pymethods]
impl Whitespace {
    fn __str__(&self) -> String {
        self.to_string()
    }
}

fn display_block(block: &[Node]) -> String {
    todo!()
}

impl<'py> pyo3::FromPyObject<'py> for Box<BooleanExpression> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        ob.extract::<BooleanExpression>().map(Box::new)
    }
}

impl pyo3::IntoPy<pyo3::PyObject> for Box<BooleanExpression> {
    fn into_py(self, py: pyo3::Python<'_>) -> pyo3::PyObject {
        (*self).into_py(py)
    }
}
