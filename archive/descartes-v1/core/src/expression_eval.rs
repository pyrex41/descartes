//! Expression Evaluator for Debugger Conditional Breakpoints
//!
//! This module provides a simple expression evaluator for use in:
//! - Conditional breakpoint evaluation
//! - Debug command expression evaluation
//! - Context inspection queries
//!
//! # Supported Expressions
//!
//! - **JSON Path Access**: `state.agent.name`, `context.variables.foo`
//! - **Comparisons**: `==`, `!=`, `>`, `<`, `>=`, `<=`
//! - **Boolean Operators**: `&&`, `||`, `!`
//! - **Numeric Operations**: `+`, `-`, `*`, `/`
//! - **Literals**: strings, numbers, booleans, null
//!
//! # Example
//!
//! ```rust
//! use descartes_core::expression_eval::{ExpressionEvaluator, EvalContext};
//! use serde_json::json;
//!
//! let context = EvalContext::new()
//!     .with_variable("count", json!(10))
//!     .with_variable("status", json!("running"));
//!
//! let evaluator = ExpressionEvaluator::new();
//!
//! // Simple comparison
//! let result = evaluator.evaluate("count > 5", &context).unwrap();
//! assert_eq!(result, json!(true));
//!
//! // String comparison
//! let result = evaluator.evaluate("status == \"running\"", &context).unwrap();
//! assert_eq!(result, json!(true));
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during expression evaluation
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum EvalError {
    #[error("Parse error at position {position}: {message}")]
    ParseError { position: usize, message: String },

    #[error("Unknown variable: {0}")]
    UnknownVariable(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Empty expression")]
    EmptyExpression,
}

pub type EvalResult<T> = Result<T, EvalError>;

// ============================================================================
// EVALUATION CONTEXT
// ============================================================================

/// Context for expression evaluation containing variables and state
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// Variables available for evaluation
    pub variables: HashMap<String, Value>,

    /// Nested state objects (e.g., "state", "context", "agent")
    pub nested: HashMap<String, Value>,
}

impl EvalContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable to the context
    pub fn with_variable(mut self, name: &str, value: Value) -> Self {
        self.variables.insert(name.to_string(), value);
        self
    }

    /// Add a nested object (like "state" or "context")
    pub fn with_nested(mut self, name: &str, value: Value) -> Self {
        self.nested.insert(name.to_string(), value);
        self
    }

    /// Get a value by path (e.g., "state.agent.status" or just "count")
    pub fn get(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return None;
        }

        // First, check if it's a simple variable
        if parts.len() == 1 {
            return self.variables.get(parts[0]);
        }

        // Check if first part is a nested object
        if let Some(root) = self.nested.get(parts[0]) {
            return Self::traverse_path(root, &parts[1..]);
        }

        // Try as a simple variable with dots (unlikely but possible)
        self.variables.get(path)
    }

    /// Traverse a JSON path
    fn traverse_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
        if path.is_empty() {
            return Some(value);
        }

        match value {
            Value::Object(map) => {
                if let Some(next) = map.get(path[0]) {
                    Self::traverse_path(next, &path[1..])
                } else {
                    None
                }
            }
            Value::Array(arr) => {
                // Allow numeric indexing into arrays
                if let Ok(index) = path[0].parse::<usize>() {
                    if let Some(next) = arr.get(index) {
                        Self::traverse_path(next, &path[1..])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Set a variable
    pub fn set(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Set a nested object
    pub fn set_nested(&mut self, name: &str, value: Value) {
        self.nested.insert(name.to_string(), value);
    }
}

// ============================================================================
// AST TYPES
// ============================================================================

/// AST node for parsed expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value (number, string, bool, null)
    Literal(Value),

    /// Variable or path reference (e.g., "count" or "state.agent.status")
    Variable(String),

    /// Binary operation (left op right)
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    /// Unary operation (op expr)
    Unary { op: UnaryOp, expr: Box<Expr> },

    /// Parenthesized expression
    Group(Box<Expr>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
}

impl BinaryOp {
    fn precedence(&self) -> u8 {
        match self {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::Ne => 3,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 4,
            BinaryOp::Add | BinaryOp::Sub => 5,
            BinaryOp::Mul | BinaryOp::Div => 6,
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

// ============================================================================
// TOKENIZER
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Literals
    Number(f64),
    String(String),
    Bool(bool),
    Null,

    // Identifier (variable name or path)
    Ident(String),

    // Operators
    Eq,       // ==
    Ne,       // !=
    Lt,       // <
    Le,       // <=
    Gt,       // >
    Ge,       // >=
    And,      // &&
    Or,       // ||
    Not,      // !
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    LParen,   // (
    RParen,   // )
    Dot,      // .

    // End of input
    Eof,
}

struct Tokenizer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    current_pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            current_pos: 0,
        }
    }

    fn next_token(&mut self) -> EvalResult<Token> {
        self.skip_whitespace();

        let (pos, ch) = match self.chars.next() {
            Some((pos, ch)) => {
                self.current_pos = pos;
                (pos, ch)
            }
            None => return Ok(Token::Eof),
        };

        match ch {
            // Operators and punctuation
            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash),
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            '.' => Ok(Token::Dot),

            // Two-character operators
            '=' => {
                if self.peek_char() == Some('=') {
                    self.chars.next();
                    Ok(Token::Eq)
                } else {
                    Err(EvalError::ParseError {
                        position: pos,
                        message: "Expected '==' for equality comparison".to_string(),
                    })
                }
            }
            '!' => {
                if self.peek_char() == Some('=') {
                    self.chars.next();
                    Ok(Token::Ne)
                } else {
                    Ok(Token::Not)
                }
            }
            '<' => {
                if self.peek_char() == Some('=') {
                    self.chars.next();
                    Ok(Token::Le)
                } else {
                    Ok(Token::Lt)
                }
            }
            '>' => {
                if self.peek_char() == Some('=') {
                    self.chars.next();
                    Ok(Token::Ge)
                } else {
                    Ok(Token::Gt)
                }
            }
            '&' => {
                if self.peek_char() == Some('&') {
                    self.chars.next();
                    Ok(Token::And)
                } else {
                    Err(EvalError::ParseError {
                        position: pos,
                        message: "Expected '&&' for logical AND".to_string(),
                    })
                }
            }
            '|' => {
                if self.peek_char() == Some('|') {
                    self.chars.next();
                    Ok(Token::Or)
                } else {
                    Err(EvalError::ParseError {
                        position: pos,
                        message: "Expected '||' for logical OR".to_string(),
                    })
                }
            }

            // String literals
            '"' => self.read_string(pos),

            // Numbers
            '0'..='9' => self.read_number(pos, ch),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.read_identifier(pos, ch),

            _ => Err(EvalError::ParseError {
                position: pos,
                message: format!("Unexpected character: '{}'", ch),
            }),
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn skip_whitespace(&mut self) {
        while let Some((_, ch)) = self.chars.peek() {
            if ch.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self, _start: usize) -> EvalResult<Token> {
        let mut s = String::new();

        loop {
            match self.chars.next() {
                Some((_, '"')) => return Ok(Token::String(s)),
                Some((_, '\\')) => {
                    // Handle escape sequences
                    match self.chars.next() {
                        Some((_, 'n')) => s.push('\n'),
                        Some((_, 't')) => s.push('\t'),
                        Some((_, 'r')) => s.push('\r'),
                        Some((_, '\\')) => s.push('\\'),
                        Some((_, '"')) => s.push('"'),
                        Some((pos, c)) => {
                            return Err(EvalError::ParseError {
                                position: pos,
                                message: format!("Unknown escape sequence: \\{}", c),
                            })
                        }
                        None => {
                            return Err(EvalError::ParseError {
                                position: self.input.len(),
                                message: "Unterminated string".to_string(),
                            })
                        }
                    }
                }
                Some((_, ch)) => s.push(ch),
                None => {
                    return Err(EvalError::ParseError {
                        position: self.input.len(),
                        message: "Unterminated string".to_string(),
                    })
                }
            }
        }
    }

    fn read_number(&mut self, start: usize, first: char) -> EvalResult<Token> {
        let mut s = String::new();
        s.push(first);

        while let Some((_, ch)) = self.chars.peek() {
            if ch.is_ascii_digit() || *ch == '.' {
                s.push(*ch);
                self.chars.next();
            } else {
                break;
            }
        }

        s.parse::<f64>()
            .map(Token::Number)
            .map_err(|_| EvalError::ParseError {
                position: start,
                message: format!("Invalid number: {}", s),
            })
    }

    fn read_identifier(&mut self, _start: usize, first: char) -> EvalResult<Token> {
        let mut s = String::new();
        s.push(first);

        while let Some((_, ch)) = self.chars.peek() {
            if ch.is_alphanumeric() || *ch == '_' {
                s.push(*ch);
                self.chars.next();
            } else {
                break;
            }
        }

        // Check for keywords
        match s.as_str() {
            "true" => Ok(Token::Bool(true)),
            "false" => Ok(Token::Bool(false)),
            "null" => Ok(Token::Null),
            _ => Ok(Token::Ident(s)),
        }
    }
}

// ============================================================================
// PARSER
// ============================================================================

struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
    current: Token,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> EvalResult<Self> {
        let mut tokenizer = Tokenizer::new(input);
        let current = tokenizer.next_token()?;
        Ok(Self { tokenizer, current })
    }

    fn parse(&mut self) -> EvalResult<Expr> {
        if self.current == Token::Eof {
            return Err(EvalError::EmptyExpression);
        }
        self.parse_expression(0)
    }

    fn advance(&mut self) -> EvalResult<()> {
        self.current = self.tokenizer.next_token()?;
        Ok(())
    }

    fn parse_expression(&mut self, min_precedence: u8) -> EvalResult<Expr> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match &self.current {
                Token::Eq => BinaryOp::Eq,
                Token::Ne => BinaryOp::Ne,
                Token::Lt => BinaryOp::Lt,
                Token::Le => BinaryOp::Le,
                Token::Gt => BinaryOp::Gt,
                Token::Ge => BinaryOp::Ge,
                Token::And => BinaryOp::And,
                Token::Or => BinaryOp::Or,
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };

            if op.precedence() < min_precedence {
                break;
            }

            self.advance()?;
            let right = self.parse_expression(op.precedence() + 1)?;

            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> EvalResult<Expr> {
        match &self.current {
            Token::Not => {
                self.advance()?;
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            Token::Minus => {
                self.advance()?;
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> EvalResult<Expr> {
        let expr = match &self.current {
            Token::Number(n) => {
                let n = *n;
                self.advance()?;
                Expr::Literal(Value::Number(
                    serde_json::Number::from_f64(n).unwrap_or_else(|| serde_json::Number::from(0)),
                ))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance()?;
                Expr::Literal(Value::String(s))
            }
            Token::Bool(b) => {
                let b = *b;
                self.advance()?;
                Expr::Literal(Value::Bool(b))
            }
            Token::Null => {
                self.advance()?;
                Expr::Literal(Value::Null)
            }
            Token::Ident(name) => {
                let mut path = name.clone();
                self.advance()?;

                // Handle dot notation for paths
                while self.current == Token::Dot {
                    self.advance()?;
                    if let Token::Ident(next) = &self.current {
                        path.push('.');
                        path.push_str(next);
                        self.advance()?;
                    } else {
                        return Err(EvalError::ParseError {
                            position: self.tokenizer.current_pos,
                            message: "Expected identifier after '.'".to_string(),
                        });
                    }
                }

                Expr::Variable(path)
            }
            Token::LParen => {
                self.advance()?;
                let expr = self.parse_expression(0)?;
                if self.current != Token::RParen {
                    return Err(EvalError::ParseError {
                        position: self.tokenizer.current_pos,
                        message: "Expected ')'".to_string(),
                    });
                }
                self.advance()?;
                Expr::Group(Box::new(expr))
            }
            _ => {
                return Err(EvalError::ParseError {
                    position: self.tokenizer.current_pos,
                    message: format!("Unexpected token: {:?}", self.current),
                });
            }
        };

        Ok(expr)
    }
}

// ============================================================================
// EVALUATOR
// ============================================================================

/// Expression evaluator for debugger conditions
#[derive(Debug, Clone, Default)]
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Create a new expression evaluator
    pub fn new() -> Self {
        Self
    }

    /// Parse an expression into an AST
    pub fn parse(&self, expression: &str) -> EvalResult<Expr> {
        let mut parser = Parser::new(expression)?;
        parser.parse()
    }

    /// Evaluate an expression against the given context
    pub fn evaluate(&self, expression: &str, context: &EvalContext) -> EvalResult<Value> {
        let ast = self.parse(expression)?;
        self.eval_expr(&ast, context)
    }

    /// Evaluate an expression and coerce result to boolean
    pub fn evaluate_bool(&self, expression: &str, context: &EvalContext) -> EvalResult<bool> {
        let result = self.evaluate(expression, context)?;
        Ok(self.to_bool(&result))
    }

    fn eval_expr(&self, expr: &Expr, context: &EvalContext) -> EvalResult<Value> {
        match expr {
            Expr::Literal(v) => Ok(v.clone()),

            Expr::Variable(path) => context
                .get(path)
                .cloned()
                .ok_or_else(|| EvalError::UnknownVariable(path.clone())),

            Expr::Binary { left, op, right } => {
                let left_val = self.eval_expr(left, context)?;
                let right_val = self.eval_expr(right, context)?;
                self.eval_binary(*op, &left_val, &right_val)
            }

            Expr::Unary { op, expr } => {
                let val = self.eval_expr(expr, context)?;
                self.eval_unary(*op, &val)
            }

            Expr::Group(inner) => self.eval_expr(inner, context),
        }
    }

    fn eval_binary(&self, op: BinaryOp, left: &Value, right: &Value) -> EvalResult<Value> {
        match op {
            // Comparison operators - use numeric comparison for numbers
            BinaryOp::Eq => self.compare_equality(left, right, false),
            BinaryOp::Ne => self.compare_equality(left, right, true),
            BinaryOp::Lt => self.compare_values(left, right, |a, b| a < b),
            BinaryOp::Le => self.compare_values(left, right, |a, b| a <= b),
            BinaryOp::Gt => self.compare_values(left, right, |a, b| a > b),
            BinaryOp::Ge => self.compare_values(left, right, |a, b| a >= b),

            // Logical operators
            BinaryOp::And => Ok(Value::Bool(self.to_bool(left) && self.to_bool(right))),
            BinaryOp::Or => Ok(Value::Bool(self.to_bool(left) || self.to_bool(right))),

            // Arithmetic operators
            BinaryOp::Add => self.numeric_op(left, right, |a, b| a + b),
            BinaryOp::Sub => self.numeric_op(left, right, |a, b| a - b),
            BinaryOp::Mul => self.numeric_op(left, right, |a, b| a * b),
            BinaryOp::Div => {
                let r = self.to_number(right)?;
                if r == 0.0 {
                    return Err(EvalError::DivisionByZero);
                }
                let l = self.to_number(left)?;
                Ok(self.number_to_value(l / r))
            }
        }
    }

    fn eval_unary(&self, op: UnaryOp, val: &Value) -> EvalResult<Value> {
        match op {
            UnaryOp::Not => Ok(Value::Bool(!self.to_bool(val))),
            UnaryOp::Neg => {
                let n = self.to_number(val)?;
                Ok(self.number_to_value(-n))
            }
        }
    }

    /// Compare values for equality, handling numeric types specially
    fn compare_equality(&self, left: &Value, right: &Value, negate: bool) -> EvalResult<Value> {
        let result = match (left, right) {
            // For numbers, compare as f64 to handle int/float mixing
            (Value::Number(a), Value::Number(b)) => {
                let a = a.as_f64().unwrap_or(f64::NAN);
                let b = b.as_f64().unwrap_or(f64::NAN);
                a == b
            }
            // For other types, use standard equality
            _ => left == right,
        };
        Ok(Value::Bool(if negate { !result } else { result }))
    }

    fn compare_values<F>(&self, left: &Value, right: &Value, cmp: F) -> EvalResult<Value>
    where
        F: Fn(f64, f64) -> bool,
    {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                let a = a.as_f64().unwrap_or(0.0);
                let b = b.as_f64().unwrap_or(0.0);
                Ok(Value::Bool(cmp(a, b)))
            }
            (Value::String(a), Value::String(b)) => {
                // Lexicographic comparison for strings
                Ok(Value::Bool(if a > b {
                    cmp(1.0, 0.0)
                } else if a < b {
                    cmp(0.0, 1.0)
                } else {
                    cmp(0.0, 0.0)
                }))
            }
            _ => Err(EvalError::TypeError(format!(
                "Cannot compare {:?} with {:?}",
                left, right
            ))),
        }
    }

    fn numeric_op<F>(&self, left: &Value, right: &Value, op: F) -> EvalResult<Value>
    where
        F: Fn(f64, f64) -> f64,
    {
        let l = self.to_number(left)?;
        let r = self.to_number(right)?;
        Ok(self.number_to_value(op(l, r)))
    }

    fn to_number(&self, val: &Value) -> EvalResult<f64> {
        match val {
            Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Null => Ok(0.0),
            Value::String(s) => s.parse::<f64>().map_err(|_| {
                EvalError::TypeError(format!("Cannot convert string '{}' to number", s))
            }),
            _ => Err(EvalError::TypeError(format!(
                "Cannot convert {:?} to number",
                val
            ))),
        }
    }

    fn to_bool(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
        }
    }

    fn number_to_value(&self, n: f64) -> Value {
        serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Evaluate an expression against a context
pub fn evaluate(expression: &str, context: &EvalContext) -> EvalResult<Value> {
    ExpressionEvaluator::new().evaluate(expression, context)
}

/// Evaluate an expression as a boolean
pub fn evaluate_bool(expression: &str, context: &EvalContext) -> EvalResult<bool> {
    ExpressionEvaluator::new().evaluate_bool(expression, context)
}

/// Create a context from a JSON value
pub fn context_from_json(value: Value) -> EvalContext {
    let mut ctx = EvalContext::new();
    if let Value::Object(map) = value {
        for (key, val) in map {
            if val.is_object() {
                ctx.nested.insert(key, val);
            } else {
                ctx.variables.insert(key, val);
            }
        }
    }
    ctx
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_literal_evaluation() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new();

        assert_eq!(eval.evaluate("42", &ctx).unwrap(), json!(42.0));
        assert_eq!(eval.evaluate("\"hello\"", &ctx).unwrap(), json!("hello"));
        assert_eq!(eval.evaluate("true", &ctx).unwrap(), json!(true));
        assert_eq!(eval.evaluate("false", &ctx).unwrap(), json!(false));
        assert_eq!(eval.evaluate("null", &ctx).unwrap(), json!(null));
    }

    #[test]
    fn test_variable_access() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new()
            .with_variable("count", json!(10))
            .with_variable("name", json!("test"));

        assert_eq!(eval.evaluate("count", &ctx).unwrap(), json!(10));
        assert_eq!(eval.evaluate("name", &ctx).unwrap(), json!("test"));
    }

    #[test]
    fn test_path_access() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new().with_nested(
            "state",
            json!({
                "agent": {
                    "status": "running",
                    "count": 5
                }
            }),
        );

        assert_eq!(
            eval.evaluate("state.agent.status", &ctx).unwrap(),
            json!("running")
        );
        assert_eq!(
            eval.evaluate("state.agent.count", &ctx).unwrap(),
            json!(5)
        );
    }

    #[test]
    fn test_comparison_operators() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new()
            .with_variable("a", json!(10))
            .with_variable("b", json!(5));

        assert_eq!(eval.evaluate("a == 10", &ctx).unwrap(), json!(true));
        assert_eq!(eval.evaluate("a != b", &ctx).unwrap(), json!(true));
        assert_eq!(eval.evaluate("a > b", &ctx).unwrap(), json!(true));
        assert_eq!(eval.evaluate("a < b", &ctx).unwrap(), json!(false));
        assert_eq!(eval.evaluate("a >= 10", &ctx).unwrap(), json!(true));
        assert_eq!(eval.evaluate("b <= 5", &ctx).unwrap(), json!(true));
    }

    #[test]
    fn test_logical_operators() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new()
            .with_variable("x", json!(true))
            .with_variable("y", json!(false));

        assert_eq!(eval.evaluate("x && y", &ctx).unwrap(), json!(false));
        assert_eq!(eval.evaluate("x || y", &ctx).unwrap(), json!(true));
        assert_eq!(eval.evaluate("!y", &ctx).unwrap(), json!(true));
        assert_eq!(eval.evaluate("!x", &ctx).unwrap(), json!(false));
    }

    #[test]
    fn test_arithmetic_operators() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new()
            .with_variable("a", json!(10))
            .with_variable("b", json!(3));

        assert_eq!(eval.evaluate("a + b", &ctx).unwrap(), json!(13.0));
        assert_eq!(eval.evaluate("a - b", &ctx).unwrap(), json!(7.0));
        assert_eq!(eval.evaluate("a * b", &ctx).unwrap(), json!(30.0));
        // Division produces float
        let result = eval.evaluate("a / b", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n.as_f64().unwrap() - 3.333333).abs() < 0.001);
        }
    }

    #[test]
    fn test_operator_precedence() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new();

        // Multiplication before addition
        assert_eq!(eval.evaluate("2 + 3 * 4", &ctx).unwrap(), json!(14.0));

        // Parentheses override precedence
        assert_eq!(eval.evaluate("(2 + 3) * 4", &ctx).unwrap(), json!(20.0));

        // Comparison before logical
        assert_eq!(
            eval.evaluate("5 > 3 && 2 < 4", &ctx).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn test_unary_operators() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new().with_variable("n", json!(5));

        assert_eq!(eval.evaluate("-n", &ctx).unwrap(), json!(-5.0));
        assert_eq!(eval.evaluate("-5", &ctx).unwrap(), json!(-5.0));
        assert_eq!(eval.evaluate("--n", &ctx).unwrap(), json!(5.0));
    }

    #[test]
    fn test_complex_expressions() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new()
            .with_variable("count", json!(10))
            .with_variable("max", json!(100))
            .with_nested(
                "state",
                json!({
                    "running": true,
                    "healthy": true
                }),
            );

        // Complex condition
        assert_eq!(
            eval.evaluate("count > 5 && count < max", &ctx).unwrap(),
            json!(true)
        );

        // Path with logical
        assert_eq!(
            eval.evaluate("state.running && state.healthy", &ctx)
                .unwrap(),
            json!(true)
        );
    }

    #[test]
    fn test_error_handling() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new();

        // Unknown variable
        assert!(matches!(
            eval.evaluate("unknown", &ctx),
            Err(EvalError::UnknownVariable(_))
        ));

        // Division by zero
        assert!(matches!(
            eval.evaluate("10 / 0", &ctx),
            Err(EvalError::DivisionByZero)
        ));

        // Empty expression
        assert!(matches!(
            eval.evaluate("", &ctx),
            Err(EvalError::EmptyExpression)
        ));
    }

    #[test]
    fn test_string_comparison() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new().with_variable("status", json!("running"));

        assert_eq!(
            eval.evaluate("status == \"running\"", &ctx).unwrap(),
            json!(true)
        );
        assert_eq!(
            eval.evaluate("status != \"stopped\"", &ctx).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn test_evaluate_bool() {
        let eval = ExpressionEvaluator::new();
        let ctx = EvalContext::new().with_variable("count", json!(10));

        assert!(eval.evaluate_bool("count > 5", &ctx).unwrap());
        assert!(!eval.evaluate_bool("count < 5", &ctx).unwrap());
        assert!(eval.evaluate_bool("true", &ctx).unwrap());
        assert!(!eval.evaluate_bool("false", &ctx).unwrap());
    }

    #[test]
    fn test_context_from_json() {
        let json_ctx = json!({
            "count": 10,
            "name": "test",
            "state": {
                "running": true
            }
        });

        let ctx = context_from_json(json_ctx);
        let eval = ExpressionEvaluator::new();

        assert_eq!(eval.evaluate("count", &ctx).unwrap(), json!(10));
        assert_eq!(eval.evaluate("name", &ctx).unwrap(), json!("test"));
        assert_eq!(
            eval.evaluate("state.running", &ctx).unwrap(),
            json!(true)
        );
    }
}
