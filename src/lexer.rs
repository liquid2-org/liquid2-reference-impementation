use std::{collections::HashMap, ops::RangeInclusive};

use pest::{iterators::Pair, iterators::Pairs, Parser};
use pest_derive::Parser;

use crate::errors::LiquidError;
use crate::markup::{Markup, RangeArgument, Token, Whitespace};
use crate::query::{
    ComparisonOperator, FilterExpression, LogicalOperator, Query, Segment, Selector,
};
use crate::unescape::unescape;

#[derive(Parser)]
#[grammar = "markup.pest"]
struct Liquid;

pub struct Lexer {
    pub query_parser: QueryParser,
}

impl Lexer {
    pub fn new() -> Self {
        Lexer {
            query_parser: QueryParser::new(),
        }
    }

    pub fn dump(&self, source: &str) {
        let elements = Liquid::parse(Rule::markup, source);
        println!("{:#?}", elements);
    }

    pub fn tokenize(&self, source: &str) -> Result<Vec<Markup>, LiquidError> {
        let pairs = Liquid::parse(Rule::markup, source)
            .map_err(|err| LiquidError::syntax(err.to_string()))?;

        let tokens: Result<Vec<_>, _> = pairs.into_iter().map(|p| self.markup(p)).collect();
        tokens
    }

    pub fn parse_query(&self, path: &str) -> Result<Query, LiquidError> {
        let mut pairs =
            Liquid::parse(Rule::query, path).map_err(|err| LiquidError::syntax(err.to_string()))?;
        self.query_parser.parse(pairs.next().unwrap().into_inner())
    }

    pub fn parse_jsonpath_query(&self, path: &str) -> Result<Query, LiquidError> {
        let mut pairs = Liquid::parse(Rule::_jsonpath, path)
            .map_err(|err| LiquidError::syntax(err.to_string()))?;
        self.query_parser.parse(pairs.next().unwrap().into_inner())
    }

    pub fn dump_query(&self, path: &str) {
        let pairs = Liquid::parse(Rule::_jsonpath, path);
        println!("{:#?}", pairs)
    }

    fn markup(&self, pair: Pair<Rule>) -> Result<Markup, LiquidError> {
        match pair.as_rule() {
            Rule::content => self.parse_content(pair),
            Rule::raw => self.parse_raw(pair),
            Rule::comment => self.parse_comment(pair),
            Rule::output => self.parse_output(pair),
            Rule::tag => self.parse_tag(pair),
            Rule::EOI => Ok(Markup::EOI {}),
            _ => unreachable!(),
        }
    }

    fn parse_content(&self, pair: Pair<Rule>) -> Result<Markup, LiquidError> {
        let span = pair.as_span();
        Ok(Markup::Content {
            span: (span.start(), span.end()),
            text: pair.as_str().to_owned(),
        })
    }

    fn parse_raw(&self, pair: Pair<Rule>) -> Result<Markup, LiquidError> {
        let span = pair.as_span();
        let mut it = pair.into_inner();
        let wc_left = Whitespace::from_str(it.next().unwrap().as_str());
        let wc_right = Whitespace::from_str(it.next().unwrap().as_str());
        let text = it.next().unwrap().as_str().to_owned();
        let end_wc_left = Whitespace::from_str(it.next().unwrap().as_str());
        let end_wc_right = Whitespace::from_str(it.next().unwrap().as_str());
        Ok(Markup::Raw {
            span: (span.start(), span.end()),
            wc: (wc_left, wc_right, end_wc_left, end_wc_right),
            text,
        })
    }

    fn parse_comment(&self, pair: Pair<Rule>) -> Result<Markup, LiquidError> {
        let span = pair.as_span();
        let mut it = pair.into_inner();
        let hashes = it.next().unwrap().as_str().to_owned();
        let wc_left = Whitespace::from_str(it.next().unwrap().as_str());
        let text = it.next().unwrap().as_str().to_owned();
        let wc_right = Whitespace::from_str(it.next().unwrap().as_str());

        Ok(Markup::Comment {
            span: (span.start(), span.end()),
            wc: (wc_left, wc_right),
            hashes,
            text,
        })
    }

    fn parse_output(&self, pair: Pair<Rule>) -> Result<Markup, LiquidError> {
        let span = pair.as_span();
        let mut it = pair.into_inner();
        let wc_left = Whitespace::from_str(it.next().unwrap().as_str());

        let mut tokens: Vec<Token> = Vec::new();
        while it.peek().is_some_and(|p| p.as_rule() != Rule::WC) {
            tokens.push(self.parse_expr_token(it.next().unwrap())?);
        }

        let wc_right = Whitespace::from_str(it.next().unwrap().as_str());

        Ok(Markup::Output {
            span: (span.start(), span.end()),
            wc: (wc_left, wc_right),
            expression: tokens,
        })
    }

    fn parse_tag(&self, pair: Pair<Rule>) -> Result<Markup, LiquidError> {
        let span = pair.as_span();
        let mut it = pair.into_inner();
        let wc_left = Whitespace::from_str(it.next().unwrap().as_str());
        let name = it.next().unwrap().as_str().to_owned();
        let mut tokens: Option<Vec<Token>> = None;

        // Don't populate Tag.expression with an empty vec.
        if it.peek().is_some_and(|p| p.as_rule() != Rule::WC) {
            let mut tokens_ = Vec::new();
            while it.peek().is_some_and(|p| p.as_rule() != Rule::WC) {
                tokens_.push(self.parse_expr_token(it.next().unwrap())?);
            }
            tokens = Some(tokens_);
        }

        let wc_right = Whitespace::from_str(it.next().unwrap().as_str());

        Ok(Markup::Tag {
            span: (span.start(), span.end()),
            name,
            wc: (wc_left, wc_right),
            expression: tokens,
        })
    }

    fn parse_expr_token(&self, pair: Pair<Rule>) -> Result<Token, LiquidError> {
        let line_col = pair.line_col();
        Ok(match pair.as_rule() {
            Rule::symbol => match pair.as_str() {
                "==" => Token::Eq { line_col },
                "!=" | "<>" => Token::Ne { line_col },
                ">=" => Token::Ge { line_col },
                "<=" => Token::Le { line_col },
                ">" => Token::Gt { line_col },
                "<" => Token::Lt { line_col },
                ":" => Token::Colon { line_col },
                "||" => Token::DoublePipe { line_col },
                "|" => Token::Pipe { line_col },
                "," => Token::Comma { line_col },
                "(" => Token::LeftParen { line_col },
                ")" => Token::RightParen { line_col },
                "=" => Token::Assign { line_col },
                _ => unreachable!(),
            },
            Rule::reserved_word => match pair.as_str() {
                "true" => Token::True_ { line_col },
                "false" => Token::False_ { line_col },
                "and" => Token::And { line_col },
                "or" => Token::Or { line_col },
                "in" => Token::In { line_col },
                "not" => Token::Not { line_col },
                "contains" => Token::Contains { line_col },
                "null" | "nil" => Token::Null { line_col },
                "if" => Token::If { line_col },
                "else" => Token::Else { line_col },
                "with" => Token::With { line_col },
                "as" => Token::As { line_col },
                "for" => Token::For { line_col },
                _ => unreachable!(),
            },
            Rule::multiline_double_quoted
            | Rule::multiline_single_quoted
            | Rule::single_quoted
            | Rule::double_quoted => Token::StringLiteral {
                line_col,
                value: pair.as_str().to_owned(),
            },
            Rule::number => self.parse_number(pair)?,
            Rule::range => self.parse_range(pair)?,
            Rule::query => Token::Query {
                line_col,
                path: self.query_parser.parse(pair.into_inner())?,
            },
            Rule::word => Token::Word {
                line_col,
                value: pair.as_str().to_owned(),
            },
            _ => unreachable!("{:#?}", pair),
        })
    }

    fn parse_number(&self, expr: Pair<Rule>) -> Result<Token, LiquidError> {
        let line_col = expr.line_col();

        if expr.as_str() == "-0" {
            return Ok(Token::IntegerLiteral { line_col, value: 0 });
        }

        let mut it = expr.into_inner();
        let mut is_float = false;
        let mut n = it.next().unwrap().as_str().to_string(); // int

        if let Some(pair) = it.next() {
            match pair.as_rule() {
                Rule::frac => {
                    is_float = true;
                    n.push_str(pair.as_str());
                }
                Rule::exp => {
                    let exp_str = pair.as_str();
                    if exp_str.contains('-') {
                        is_float = true;
                    }
                    n.push_str(exp_str);
                }
                _ => unreachable!(),
            }
        }

        if let Some(pair) = it.next() {
            let exp_str = pair.as_str();
            if exp_str.contains('-') {
                is_float = true;
            }
            n.push_str(exp_str);
        }

        if is_float {
            Ok(Token::FloatLiteral {
                line_col,
                value: n
                    .parse::<f64>()
                    .map_err(|_| LiquidError::syntax(String::from("invalid float literal")))?,
            })
        } else {
            Ok(Token::IntegerLiteral {
                line_col,
                value: n
                    .parse::<f64>()
                    .map_err(|_| LiquidError::syntax(String::from("invalid integer literal")))?
                    as i64,
            })
        }
    }

    fn parse_range(&self, expr: Pair<Rule>) -> Result<Token, LiquidError> {
        let line_col = expr.line_col();
        let mut it = expr.into_inner();
        let start = self.parse_range_argument(it.next().unwrap())?;
        let stop = self.parse_range_argument(it.next().unwrap())?;
        Ok(Token::RangeLiteral {
            line_col,
            start,
            stop,
        })
    }

    fn parse_range_argument(&self, pair: Pair<Rule>) -> Result<RangeArgument, LiquidError> {
        match pair.as_rule() {
            Rule::number => match self.parse_number(pair)? {
                Token::FloatLiteral { line_col, value } => {
                    Ok(RangeArgument::FloatLiteral { line_col, value })
                }
                Token::IntegerLiteral { line_col, value } => {
                    Ok(RangeArgument::IntegerLiteral { line_col, value })
                }
                _ => unreachable!(),
            },
            Rule::query => {
                let line_col = pair.line_col();
                Ok(RangeArgument::Query {
                    line_col,
                    path: self.query_parser.parse(pair.into_inner())?,
                })
            }
            Rule::string_literal | Rule::multiline_string_literal => {
                let line_col = pair.line_col();
                Ok(RangeArgument::StringLiteral {
                    line_col,
                    value: pair.as_str().to_owned(),
                })
            }
            _ => unreachable!("{:#?}", pair),
        }
    }
}

pub struct QueryParser {
    pub index_range: RangeInclusive<i64>,
    pub functions: HashMap<String, FunctionSignature>,
}

impl QueryParser {
    pub fn new() -> Self {
        QueryParser {
            index_range: ((-2_i64).pow(53) + 1..=2_i64.pow(53) - 1),
            functions: standard_functions(),
        }
    }

    pub fn parse(&self, segments: Pairs<Rule>) -> Result<Query, LiquidError> {
        let segments: Result<Vec<_>, _> = segments
            .map(|segment| self.parse_segment(segment))
            .collect();

        Ok(Query {
            segments: segments?,
        })
    }

    fn parse_segment(&self, segment: Pair<Rule>) -> Result<Segment, LiquidError> {
        let line_col = segment.line_col();
        Ok(match segment.as_rule() {
            Rule::child_segment | Rule::implicit_root_segment => Segment::Child {
                selectors: self.parse_segment_inner(segment.into_inner().next().unwrap())?,
                line_col,
            },
            Rule::descendant_segment => Segment::Recursive {
                selectors: self.parse_segment_inner(segment.into_inner().next().unwrap())?,
                line_col,
            },
            Rule::name_segment | Rule::index_segment | Rule::implicit_root_name_segment => {
                Segment::Child {
                    selectors: vec![self.parse_selector(segment.into_inner().next().unwrap())?],
                    line_col,
                }
            }
            Rule::EOI => Segment::Eoi {},
            _ => unreachable!("{:#?}", segment),
        })
    }

    fn parse_segment_inner(&self, segment: Pair<Rule>) -> Result<Vec<Selector>, LiquidError> {
        Ok(match segment.as_rule() {
            Rule::bracketed_selection => {
                let seg: Result<Vec<_>, _> = segment
                    .into_inner()
                    .map(|selector| self.parse_selector(selector))
                    .collect();
                seg?
            }
            Rule::wildcard_selector => vec![Selector::Wild {
                line_col: segment.line_col(),
            }],
            Rule::member_name_shorthand => vec![Selector::Name {
                // for child_segment
                name: segment.as_str().to_owned(),
                line_col: segment.line_col(),
            }],
            _ => unreachable!(),
        })
    }

    fn parse_selector(&self, selector: Pair<Rule>) -> Result<Selector, LiquidError> {
        let line_col = selector.line_col();
        Ok(match selector.as_rule() {
            Rule::double_quoted => Selector::Name {
                name: unescape(selector.as_str(), &line_col)?,
                line_col,
            },
            Rule::single_quoted => Selector::Name {
                name: unescape(&selector.as_str().replace("\\'", "'"), &line_col)?,
                line_col,
            },
            Rule::wildcard_selector => Selector::Wild { line_col },
            Rule::slice_selector => self.parse_slice_selector(selector)?,
            Rule::index_selector => Selector::Index {
                index: self.parse_i_json_int(selector.as_str())?,
                line_col,
            },
            Rule::filter_selector => self.parse_filter_selector(selector)?,
            Rule::member_name_shorthand => Selector::Name {
                // for name_segment
                name: selector.as_str().to_owned(),
                line_col,
            },
            Rule::singular_query_selector => self.parse_singular_query_selector(selector)?,
            _ => unreachable!("{:#?}", selector),
        })
    }

    fn parse_slice_selector(&self, selector: Pair<Rule>) -> Result<Selector, LiquidError> {
        let mut start: Option<i64> = None;
        let mut stop: Option<i64> = None;
        let mut step: Option<i64> = None;
        let line_col = selector.line_col();

        for i in selector.into_inner() {
            match i.as_rule() {
                Rule::start => start = Some(self.parse_i_json_int(i.as_str())?),
                Rule::stop => stop = Some(self.parse_i_json_int(i.as_str())?),
                Rule::step => step = Some(self.parse_i_json_int(i.as_str())?),
                _ => unreachable!(),
            }
        }

        Ok(Selector::Slice {
            start,
            stop,
            step,
            line_col,
        })
    }

    fn parse_filter_selector(&self, selector: Pair<Rule>) -> Result<Selector, LiquidError> {
        let line_col = selector.line_col();
        Ok(Selector::Filter {
            expression: Box::new(
                self.parse_logical_or_expression(selector.into_inner().next().unwrap(), true)?,
            ),
            line_col,
        })
    }

    fn parse_singular_query_selector(&self, selector: Pair<Rule>) -> Result<Selector, LiquidError> {
        let line_col = selector.line_col();
        let segments: Result<Vec<_>, _> = selector
            .into_inner()
            .map(|segment| self.parse_segment(segment))
            .collect();

        Ok(Selector::SingularQuery {
            query: Box::new(Query {
                segments: segments?,
            }),
            line_col,
        })
    }

    fn parse_logical_or_expression(
        &self,
        expr: Pair<Rule>,
        assert_compared: bool,
    ) -> Result<FilterExpression, LiquidError> {
        let mut it = expr.into_inner();
        let mut or_expr = self.parse_logical_and_expression(it.next().unwrap(), assert_compared)?;

        if assert_compared {
            self.assert_compared(&or_expr)?;
        }

        for and_expr in it {
            let line_col = and_expr.line_col();
            let right = self.parse_logical_and_expression(and_expr, assert_compared)?;
            if assert_compared {
                self.assert_compared(&right)?;
            }
            or_expr = FilterExpression::Logical {
                left: Box::new(or_expr),
                operator: LogicalOperator::Or,
                right: Box::new(right),
                line_col,
            };
        }

        Ok(or_expr)
    }

    fn parse_logical_and_expression(
        &self,
        expr: Pair<Rule>,
        assert_compared: bool,
    ) -> Result<FilterExpression, LiquidError> {
        let line_col = expr.line_col();
        let mut it = expr.into_inner();
        let mut and_expr = self.parse_basic_expression(it.next().unwrap())?;

        if assert_compared {
            self.assert_compared(&and_expr)?;
        }

        for basic_expr in it {
            let right = self.parse_basic_expression(basic_expr)?;

            if assert_compared {
                self.assert_compared(&right)?;
            }

            and_expr = FilterExpression::Logical {
                left: Box::new(and_expr),
                operator: LogicalOperator::And,
                right: Box::new(right),
                line_col: line_col,
            };
        }

        Ok(and_expr)
    }

    fn parse_basic_expression(&self, expr: Pair<Rule>) -> Result<FilterExpression, LiquidError> {
        match expr.as_rule() {
            Rule::paren_expr => self.parse_paren_expression(expr),
            Rule::comparison_expr => self.parse_comparison_expression(expr),
            Rule::test_expr => self.parse_test_expression(expr),
            _ => unreachable!(),
        }
    }

    fn parse_paren_expression(&self, expr: Pair<Rule>) -> Result<FilterExpression, LiquidError> {
        let mut it = expr.into_inner();
        let p = it.next().unwrap();
        match p.as_rule() {
            Rule::logical_not_op => Ok(FilterExpression::Not {
                expression: Box::new(self.parse_logical_or_expression(it.next().unwrap(), true)?),
                line_col: p.line_col(),
            }),
            Rule::logical_or_expr => self.parse_logical_or_expression(p, true),
            _ => unreachable!(),
        }
    }

    fn parse_comparison_expression(
        &self,
        expr: Pair<Rule>,
    ) -> Result<FilterExpression, LiquidError> {
        let mut it = expr.into_inner();
        let pair = it.next().unwrap();
        let line_col = pair.line_col();
        let left = self.parse_comparable(pair)?;

        let operator = match it.next().unwrap().as_str() {
            "==" => ComparisonOperator::Eq,
            "!=" => ComparisonOperator::Ne,
            "<=" => ComparisonOperator::Le,
            ">=" => ComparisonOperator::Ge,
            "<" => ComparisonOperator::Lt,
            ">" => ComparisonOperator::Gt,
            _ => unreachable!(),
        };

        let right = self.parse_comparable(it.next().unwrap())?;
        self.assert_comparable(&left)?;
        self.assert_comparable(&right)?;

        Ok(FilterExpression::Comparison {
            left: Box::new(left),
            operator,
            right: Box::new(right),
            line_col,
        })
    }

    fn parse_comparable(&self, expr: Pair<Rule>) -> Result<FilterExpression, LiquidError> {
        let line_col = expr.line_col();
        Ok(match expr.as_rule() {
            Rule::number => self.parse_number(expr)?,
            Rule::double_quoted => FilterExpression::StringLiteral {
                value: unescape(expr.as_str(), &line_col)?,
                line_col,
            },
            Rule::single_quoted => FilterExpression::StringLiteral {
                value: unescape(&expr.as_str().replace("\\'", "'"), &line_col)?,
                line_col,
            },
            Rule::true_literal => FilterExpression::True_ { line_col },
            Rule::false_literal => FilterExpression::False_ { line_col },
            Rule::null => FilterExpression::Null { line_col },
            Rule::rel_singular_query => {
                let segments: Result<Vec<_>, _> = expr
                    .into_inner()
                    .map(|segment| self.parse_segment(segment))
                    .collect();

                FilterExpression::RelativeQuery {
                    query: Box::new(Query {
                        segments: segments?,
                    }),
                    line_col,
                }
            }
            Rule::abs_singular_query => {
                let segments: Result<Vec<_>, _> = expr
                    .into_inner()
                    .map(|segment| self.parse_segment(segment))
                    .collect();

                FilterExpression::RootQuery {
                    query: Box::new(Query {
                        segments: segments?,
                    }),
                    line_col,
                }
            }
            Rule::function_expr => self.parse_function_expression(expr)?,
            _ => unreachable!(),
        })
    }

    fn parse_number(&self, expr: Pair<Rule>) -> Result<FilterExpression, LiquidError> {
        let line_col = expr.line_col();
        if expr.as_str() == "-0" {
            return Ok(FilterExpression::Int { value: 0, line_col });
        }

        // TODO: change pest grammar to indicate positive or negative exponent?
        let mut it = expr.into_inner();
        let mut is_float = false;
        let mut n = it.next().unwrap().as_str().to_string(); // int

        if let Some(pair) = it.next() {
            match pair.as_rule() {
                Rule::frac => {
                    is_float = true;
                    n.push_str(pair.as_str());
                }
                Rule::exp => {
                    let exp_str = pair.as_str();
                    if exp_str.contains('-') {
                        is_float = true;
                    }
                    n.push_str(exp_str);
                }
                _ => unreachable!(),
            }
        }

        if let Some(pair) = it.next() {
            let exp_str = pair.as_str();
            if exp_str.contains('-') {
                is_float = true;
            }
            n.push_str(exp_str);
        }

        if is_float {
            Ok(FilterExpression::Float {
                value: n
                    .parse::<f64>()
                    .map_err(|_| LiquidError::syntax(String::from("invalid float literal")))?,
                line_col,
            })
        } else {
            Ok(FilterExpression::Int {
                value: n
                    .parse::<f64>()
                    .map_err(|_| LiquidError::syntax(String::from("invalid integer literal")))?
                    as i64,
                line_col,
            })
        }
    }

    fn parse_test_expression(&self, expr: Pair<Rule>) -> Result<FilterExpression, LiquidError> {
        let mut it = expr.into_inner();
        let pair = it.next().unwrap();
        Ok(match pair.as_rule() {
            Rule::logical_not_op => FilterExpression::Not {
                expression: Box::new(self.parse_test_expression_inner(it.next().unwrap())?),
                line_col: pair.line_col(),
            },
            _ => self.parse_test_expression_inner(pair)?,
        })
    }

    fn parse_test_expression_inner(
        &self,
        expr: Pair<Rule>,
    ) -> Result<FilterExpression, LiquidError> {
        let line_col = expr.line_col();
        Ok(match expr.as_rule() {
            Rule::rel_query => {
                let segments: Result<Vec<_>, _> = expr
                    .into_inner()
                    .map(|segment| self.parse_segment(segment))
                    .collect();

                FilterExpression::RelativeQuery {
                    query: Box::new(Query {
                        segments: segments?,
                    }),
                    line_col,
                }
            }
            Rule::root_query => {
                let segments: Result<Vec<_>, _> = expr
                    .into_inner()
                    .map(|segment| self.parse_segment(segment))
                    .collect();

                FilterExpression::RootQuery {
                    query: Box::new(Query {
                        segments: segments?,
                    }),
                    line_col,
                }
            }
            Rule::function_expr => self.parse_function_expression(expr)?,
            _ => unreachable!(),
        })
    }

    fn parse_function_expression(&self, expr: Pair<Rule>) -> Result<FilterExpression, LiquidError> {
        let mut it = expr.into_inner();
        let pair = it.next().unwrap();
        let line_col = pair.line_col();
        let name = pair.as_str();
        let args: Result<Vec<_>, _> = it.map(|ex| self.parse_function_argument(ex)).collect();

        Ok(FilterExpression::Function {
            name: name.to_string(),
            args: self.assert_well_typed(name, args?)?,
            line_col,
        })
    }

    fn parse_function_argument(&self, expr: Pair<Rule>) -> Result<FilterExpression, LiquidError> {
        let line_col = expr.line_col();
        Ok(match expr.as_rule() {
            Rule::number => self.parse_number(expr)?,
            Rule::double_quoted => FilterExpression::StringLiteral {
                value: unescape(expr.as_str(), &line_col)?,
                line_col,
            },
            Rule::single_quoted => FilterExpression::StringLiteral {
                value: unescape(&expr.as_str().replace("\\'", "'"), &line_col)?,
                line_col,
            },
            Rule::true_literal => FilterExpression::True_ { line_col },
            Rule::false_literal => FilterExpression::False_ { line_col },
            Rule::null => FilterExpression::Null { line_col },
            Rule::rel_query => {
                let segments: Result<Vec<_>, _> = expr
                    .into_inner()
                    .map(|segment| self.parse_segment(segment))
                    .collect();

                FilterExpression::RelativeQuery {
                    query: Box::new(Query {
                        segments: segments?,
                    }),
                    line_col,
                }
            }
            Rule::root_query => {
                let segments: Result<Vec<_>, _> = expr
                    .into_inner()
                    .map(|segment| self.parse_segment(segment))
                    .collect();

                FilterExpression::RootQuery {
                    query: Box::new(Query {
                        segments: segments?,
                    }),
                    line_col,
                }
            }
            Rule::logical_or_expr => self.parse_logical_or_expression(expr, false)?,
            Rule::function_expr => self.parse_function_expression(expr)?,
            _ => unreachable!(),
        })
    }

    fn parse_i_json_int(&self, value: &str) -> Result<i64, LiquidError> {
        let i = value
            .parse::<i64>()
            .map_err(|_| LiquidError::syntax(format!("index out of range `{}`", value)))?;

        if !self.index_range.contains(&i) {
            return Err(LiquidError::syntax(format!(
                "index out of range `{}`",
                value
            )));
        }

        Ok(i)
    }
    fn assert_comparable(&self, expr: &FilterExpression) -> Result<(), LiquidError> {
        // TODO: accept span/position for better errors
        match expr {
            FilterExpression::RelativeQuery { query, .. }
            | FilterExpression::RootQuery { query, .. } => {
                if !query.is_singular() {
                    Err(LiquidError::typ(String::from(
                        "non-singular query is not comparable",
                    )))
                } else {
                    Ok(())
                }
            }
            FilterExpression::Function { name, .. } => {
                if let Some(FunctionSignature {
                    return_type: ExpressionType::Value,
                    ..
                }) = self.functions.get(name)
                {
                    Ok(())
                } else {
                    Err(LiquidError::typ(format!(
                        "result of {}() is not comparable",
                        name
                    )))
                }
            }
            _ => Ok(()),
        }
    }

    fn assert_compared(&self, expr: &FilterExpression) -> Result<(), LiquidError> {
        match expr {
            FilterExpression::Function { name, .. } => {
                if let Some(FunctionSignature {
                    return_type: ExpressionType::Value,
                    ..
                }) = self.functions.get(name)
                {
                    Err(LiquidError::typ(format!(
                        "result of {}() must be compared",
                        name
                    )))
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    fn assert_well_typed(
        &self,
        func_name: &str,
        args: Vec<FilterExpression>,
    ) -> Result<Vec<FilterExpression>, LiquidError> {
        // TODO: accept span/position for better errors
        let signature = self
            .functions
            .get(func_name)
            .ok_or_else(|| LiquidError::name(format!("unknown function `{}`", func_name)))?;

        // correct number of arguments?
        if args.len() != signature.param_types.len() {
            return Err(LiquidError::typ(format!(
                "{}() takes {} argument{} but {} were given",
                func_name,
                signature.param_types.len(),
                if signature.param_types.len() > 1 {
                    "s"
                } else {
                    ""
                },
                args.len()
            )));
        }

        // correct argument types?
        for (idx, typ) in signature.param_types.iter().enumerate() {
            let arg = &args[idx];
            match typ {
                ExpressionType::Value => {
                    if !self.is_value_type(arg) {
                        return Err(LiquidError::typ(format!(
                            "argument {} of {}() must be of a 'Value' type",
                            idx + 1,
                            func_name
                        )));
                    }
                }
                ExpressionType::Logical => {
                    if !matches!(
                        arg,
                        FilterExpression::RelativeQuery { .. }
                            | FilterExpression::RootQuery { .. }
                            | FilterExpression::Logical { .. }
                            | FilterExpression::Comparison { .. },
                    ) {
                        return Err(LiquidError::typ(format!(
                            "argument {} of {}() must be of a 'Logical' type",
                            idx + 1,
                            func_name
                        )));
                    }
                }
                ExpressionType::Nodes => {
                    if !self.is_nodes_type(arg) {
                        return Err(LiquidError::typ(format!(
                            "argument {} of {}() must be of a 'Nodes' type",
                            idx + 1,
                            func_name
                        )));
                    }
                }
            }
        }

        Ok(args)
    }

    fn is_value_type(&self, expr: &FilterExpression) -> bool {
        // literals are values
        if expr.is_literal() {
            return true;
        }

        match expr {
            FilterExpression::RelativeQuery { query, .. }
            | FilterExpression::RootQuery { query, .. } => {
                // singular queries will be coerced to a value
                query.is_singular()
            }
            FilterExpression::Function { name, .. } => {
                // some functions return a value
                matches!(
                    self.functions.get(name),
                    Some(FunctionSignature {
                        return_type: ExpressionType::Value,
                        ..
                    })
                )
            }
            _ => false,
        }
    }

    fn is_nodes_type(&self, expr: &FilterExpression) -> bool {
        match expr {
            FilterExpression::RelativeQuery { .. } | FilterExpression::RootQuery { .. } => true,
            FilterExpression::Function { name, .. } => {
                matches!(
                    self.functions.get(name),
                    Some(FunctionSignature {
                        return_type: ExpressionType::Nodes,
                        ..
                    })
                )
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum ExpressionType {
    Logical,
    Nodes,
    Value,
}

pub struct FunctionSignature {
    pub param_types: Vec<ExpressionType>,
    pub return_type: ExpressionType,
}

pub fn standard_functions() -> HashMap<String, FunctionSignature> {
    let mut functions = HashMap::new();

    functions.insert(
        "count".to_owned(),
        FunctionSignature {
            param_types: vec![ExpressionType::Nodes],
            return_type: ExpressionType::Value,
        },
    );

    functions.insert(
        "length".to_owned(),
        FunctionSignature {
            param_types: vec![ExpressionType::Value],
            return_type: ExpressionType::Value,
        },
    );

    functions.insert(
        "match".to_owned(),
        FunctionSignature {
            param_types: vec![ExpressionType::Value, ExpressionType::Value],
            return_type: ExpressionType::Logical,
        },
    );

    functions.insert(
        "search".to_owned(),
        FunctionSignature {
            param_types: vec![ExpressionType::Value, ExpressionType::Value],
            return_type: ExpressionType::Logical,
        },
    );

    functions.insert(
        "value".to_owned(),
        FunctionSignature {
            param_types: vec![ExpressionType::Nodes],
            return_type: ExpressionType::Value,
        },
    );

    functions
}
