use std::collections::HashMap;

use anyhow::{Context, anyhow};
use pdx_parser_core::{helpers::SplitAtFirst, text_deserialize::TextError, text_err};

use crate::flags::parser::VariableValue;

pub trait VariableGet: Sized {
    fn get(&self, name: &str) -> Option<f64>;
    fn resolve(&self, var: &VariableValue) -> anyhow::Result<f64> {
        match var {
            VariableValue::Literal(literal) => Ok(*literal),
            VariableValue::Variable(var) => self
                .get(var)
                .ok_or(anyhow!("Could not find variable {var}")),
            VariableValue::Expression(expr) => {
                let expr = ExpressionToken::parse_stream(&expr)?;
                let expr = ExpressionNode::from_tokens(&expr)?;
                let value = expr.eval(&self)?;
                Ok(value)
            }
        }
    }
}
impl<T: VariableGet> VariableGet for &T {
    fn get(&self, name: &str) -> Option<f64> {
        (*self).get(name)
    }
}

/// Basically a `HashMap<String, f64>`, meant for resolving variables from a specific scope.
/// Generally you want [`VariableResolver`] instead since it allows propogating variables from parent scopes.
pub struct VariableScope {
    variables: HashMap<String, f64>,
}
impl VariableScope {
    pub fn new() -> Self {
        return VariableScope {
            variables: HashMap::new(),
        };
    }
    pub fn add_variable(&mut self, name: String, value: f64) {
        self.variables.insert(name, value);
    }
    pub fn with_variable(self, name: String, value: f64) -> Self {
        let mut out = self;
        out.add_variable(name, value);
        return out;
    }
    /// Creates a new scope from the given variables. Does not use any parent scopes in creating the new scope.
    pub fn from_unresolved(
        variables: impl IntoIterator<Item = (String, VariableValue)>,
    ) -> anyhow::Result<Self> {
        let scope = VariableScope::new()
            .into_resolver_root()
            .with_new_scope_from_unresolved(variables)?
            .unwrap_current_scope();
        Ok(scope)
    }
    /// Calls `VariableResolver::root(self)`. May be useful for chained calls.
    pub fn into_resolver_root(self) -> VariableResolver<'static> {
        VariableResolver::root(self)
    }
}
impl VariableGet for VariableScope {
    fn get(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }
}

/// Basically a linked list of [`VariableScope`]s, meant for resolving variables from the current and parent scopes.
pub struct VariableResolver<'a> {
    parent_resolver: Option<&'a VariableResolver<'a>>,
    scope: VariableScope,
}
impl<'a> VariableResolver<'a> {
    /// Creates a new root resolver
    pub fn root(scope: VariableScope) -> VariableResolver<'static> {
        VariableResolver {
            parent_resolver: None,
            scope,
        }
    }
    /// Returns a new resolver with the given scope added to the top of the stack
    pub fn with_scope(&'a self, scope: VariableScope) -> Self {
        VariableResolver {
            parent_resolver: Some(self),
            scope,
        }
    }
    pub fn parent(&self) -> Option<&VariableResolver<'a>> {
        self.parent_resolver
    }
    /// Gets an immutable reference to the current scope
    pub fn current_scope(&self) -> &VariableScope {
        &self.scope
    }
    pub fn unwrap_current_scope(self) -> VariableScope {
        self.scope
    }
    pub fn with_new_scope_from_unresolved(
        &'a self,
        variables: impl IntoIterator<Item = (String, VariableValue)>,
    ) -> Result<Self, anyhow::Error> {
        let mut scope = VariableScope::new();
        for (name, value) in variables {
            let value = match value {
                VariableValue::Literal(literal) => literal,
                VariableValue::Variable(var) => scope
                    .get(&var)
                    .or_else(|| self.get(&var))
                    .ok_or(anyhow!("Unknown variable {var} used for @{name}"))?,
                VariableValue::Expression(expr) => {
                    let expr = ExpressionToken::parse_stream(&expr).with_context(|| {
                        format!("While tokenizing expression [{expr}] for @{name}")
                    })?;
                    let expr = ExpressionNode::from_tokens(&expr)
                        .with_context(|| format!("Parsing AST for @{name} expression"))?;
                    let tmp = VariableResolver::root(scope);
                    let value = expr
                        .eval(&tmp)
                        .with_context(|| format!("While evaluating expression for @{name}"))?;
                    scope = tmp.unwrap_current_scope();
                    value
                }
            };
            scope.add_variable(name, value);
        }
        Ok(VariableResolver {
            parent_resolver: Some(self),
            scope,
        })
    }
}
impl VariableGet for VariableResolver<'_> {
    fn get(&self, name: &str) -> Option<f64> {
        self.scope
            .get(name)
            .or_else(|| self.parent().and_then(|parent| parent.get(name)))
    }
}

#[derive(Debug, PartialEq)]
pub enum ExpressionToken {
    Literal(f64),
    Variable(String),
    ParenOpen,
    ParenClose,
    Add,
    Sub,
    Mul,
    Div,
}
impl ExpressionToken {
    pub fn parse_stream(stream: &str) -> Result<Vec<ExpressionToken>, TextError> {
        if !stream.is_ascii() {
            return Err(text_err!("Expected ascii stream for expression parsing"));
        }
        let mut stream = stream.as_bytes();
        let mut out = Vec::new();
        loop {
            stream = stream.trim_ascii_start();
            match stream {
                [] => break,
                [b'(', rest @ ..] => {
                    stream = rest;
                    out.push(ExpressionToken::ParenOpen);
                }
                [b')', rest @ ..] => {
                    stream = rest;
                    out.push(ExpressionToken::ParenClose);
                }
                [b'+', rest @ ..] => {
                    stream = rest;
                    out.push(ExpressionToken::Add);
                }
                [b'-', rest @ ..] => {
                    stream = rest;
                    out.push(ExpressionToken::Sub);
                }
                [b'*', rest @ ..] => {
                    stream = rest;
                    out.push(ExpressionToken::Mul);
                }
                [b'/', rest @ ..] => {
                    stream = rest;
                    out.push(ExpressionToken::Div);
                }
                [b'a'..=b'z' | b'A'..=b'Z' | b'_', rest @ ..] => {
                    // hope that variable names never start with a digit
                    let (var_name, rest) = stream
                        .split_at_first_inclusive(|b| !b.is_ascii_alphanumeric() && *b != b'_')
                        .unwrap_or((stream, b""));
                    let var_name = str::from_utf8(var_name).unwrap_or_else(|_| {
                        unreachable!("We already checked the input stream is ascii")
                    });
                    stream = rest;
                    out.push(ExpressionToken::Variable(var_name.to_string()));
                }
                [b, ..] if b.is_ascii_digit() => {
                    let (num, rest) = stream
                        .split_at_first_inclusive(|b| !b.is_ascii_digit() && *b != b'.')
                        .unwrap_or((stream, b""));
                    stream = rest;
                    let num = str::from_utf8(num).unwrap_or_else(|_| {
                        unreachable!("We already checked the input stream is ascii")
                    });
                    let num = num
                        .parse()
                        .map_err(|_| text_err!("Failed to parse \"{num}\" as an f64"))?;
                    out.push(ExpressionToken::Literal(num));
                }
                rest => {
                    return Err(text_err!(
                        "Invalid variable expression token found: {:?}",
                        str::from_utf8(rest),
                    ));
                }
            }
        }
        return Ok(out);
    }
}

pub enum ExpressionNode {
    Literal(f64),
    Variable(String),
    Paren(Box<ExpressionNode>),
    Add(Vec<ExpressionNode>),
    Sub(Box<ExpressionNode>, Vec<ExpressionNode>),
    Mul(Vec<ExpressionNode>),
    Div(Box<ExpressionNode>, Vec<ExpressionNode>),
}
impl ExpressionNode {
    /// Tries to evaluate the expression to a number. Can error if a variable cannot be resolved.
    pub fn eval(&self, variables: &impl VariableGet) -> Result<f64, TextError> {
        return match self {
            ExpressionNode::Literal(value) => Ok(*value),
            ExpressionNode::Variable(var) => variables.get(var).ok_or(text_err!(
                "Variable {var} was not found in the variable resolver"
            )),
            ExpressionNode::Paren(node) => node.eval(variables),
            ExpressionNode::Add(terms) => {
                let mut out = 0.0;
                for term in terms {
                    out += term.eval(variables)?;
                }
                return Ok(out);
            }
            ExpressionNode::Sub(from, terms) => {
                let mut out = from.eval(variables)?;
                for term in terms {
                    out -= term.eval(variables)?;
                }
                return Ok(out);
            }
            ExpressionNode::Mul(terms) => {
                let mut out = 1.0;
                for term in terms {
                    out *= term.eval(variables)?;
                }
                return Ok(out);
            }
            ExpressionNode::Div(from, terms) => {
                let mut out = from.eval(variables)?;
                for term in terms {
                    out /= term.eval(variables)?;
                }
                return Ok(out);
            }
        };
    }
    pub fn from_tokens(tokens: &[ExpressionToken]) -> Result<Self, TextError> {
        let (node, rest) = Self::from_tokens_inner(tokens)?;
        if let Some(rest) = rest.first() {
            assert!(matches!(rest, ExpressionToken::ParenClose));
            return Err(text_err!("Unmatched parenthesis"));
        }
        return Ok(node);
    }
    fn from_tokens_inner<'a>(
        tokens: &'a [ExpressionToken],
    ) -> Result<(Self, &'a [ExpressionToken]), TextError> {
        let (mut last_node, mut tokens) = Self::take_value(tokens)?;
        loop {
            match (tokens, last_node) {
                (rest @ ([] | [ExpressionToken::ParenClose, ..]), last) => {
                    return Ok((last, rest));
                }
                ([ExpressionToken::Add, rest @ ..], last) => {
                    let (value, rest) = Self::take_value(rest)?;
                    tokens = rest;
                    if let ExpressionNode::Add(mut terms) = last {
                        terms.push(value);
                        last_node = ExpressionNode::Add(terms);
                    } else {
                        last_node = ExpressionNode::Add(vec![last, value]);
                    }
                }
                ([ExpressionToken::Sub, rest @ ..], last) => {
                    let (value, rest) = Self::take_value(rest)?;
                    tokens = rest;
                    if let ExpressionNode::Sub(from, mut terms) = last {
                        terms.push(value);
                        last_node = ExpressionNode::Sub(from, terms);
                    } else {
                        last_node = ExpressionNode::Sub(Box::new(last), vec![value]);
                    }
                }
                ([ExpressionToken::Mul, rest @ ..], last) => {
                    let (value, rest) = Self::take_value(rest)?;
                    tokens = rest;
                    last_node = Self::recurse_mul(last, value)?;
                }
                ([ExpressionToken::Div, rest @ ..], last) => {
                    let (value, rest) = Self::take_value(rest)?;
                    tokens = rest;
                    last_node = Self::recurse_div(last, value)?;
                }
                ([_value, ..], _) => {
                    return Err(text_err!(
                        "A value cannot be directly followed by another value"
                    ));
                }
            }
        }
    }
    fn take_value<'a>(
        tokens: &'a [ExpressionToken],
    ) -> Result<(Self, &'a [ExpressionToken]), TextError> {
        match tokens {
            [] => Err(text_err!(
                "Unexpected end of expression when value was expected"
            )),
            [ExpressionToken::Literal(value), rest @ ..] => {
                return Ok((Self::Literal(*value), rest));
            }
            [ExpressionToken::Variable(var), rest @ ..] => {
                return Ok((Self::Variable(var.to_string()), rest));
            }
            [ExpressionToken::ParenOpen, rest @ ..] => {
                let (node, rest) = Self::from_tokens_inner(rest)?;
                let rest = rest
                    .strip_prefix(&[ExpressionToken::ParenClose])
                    .ok_or(text_err!(
                        "Unbalanced parentheses - subexpression was not closed"
                    ))?;
                return Ok((Self::Paren(Box::new(node)), rest));
            }
            _ => Err(text_err!("Expected a value, but found something else")),
        }
    }
    fn recurse_mul(
        node: ExpressionNode,
        value: ExpressionNode,
    ) -> Result<ExpressionNode, TextError> {
        match node {
            ExpressionNode::Mul(mut terms) => {
                terms.push(value);
                return Ok(ExpressionNode::Mul(terms));
            }
            ExpressionNode::Add(mut terms) => {
                let last = terms
                    .pop()
                    .ok_or(text_err!("INTERNAL ERROR: expression node add was empty"))?;
                terms.push(Self::recurse_mul(last, value)?);
                return Ok(ExpressionNode::Add(terms));
            }
            ExpressionNode::Sub(from, mut terms) => {
                let last = terms
                    .pop()
                    .ok_or(text_err!("INTERNAL ERROR: expression node sub was empty"))?;
                terms.push(Self::recurse_mul(last, value)?);
                return Ok(ExpressionNode::Sub(from, terms));
            }
            other => {
                return Ok(ExpressionNode::Mul(vec![other, value]));
            }
        }
    }
    fn recurse_div(
        node: ExpressionNode,
        value: ExpressionNode,
    ) -> Result<ExpressionNode, TextError> {
        match node {
            ExpressionNode::Div(from, mut terms) => {
                terms.push(value);
                return Ok(ExpressionNode::Div(from, terms));
            }
            ExpressionNode::Add(mut terms) => {
                let last = terms
                    .pop()
                    .ok_or(text_err!("INTERNAL ERROR: expression node add was empty"))?;
                terms.push(Self::recurse_div(last, value)?);
                return Ok(ExpressionNode::Add(terms));
            }
            ExpressionNode::Sub(from, mut terms) => {
                let last = terms
                    .pop()
                    .ok_or(text_err!("INTERNAL ERROR: expression node sub was empty"))?;
                terms.push(Self::recurse_div(last, value)?);
                return Ok(ExpressionNode::Sub(from, terms));
            }
            other => {
                return Ok(ExpressionNode::Div(Box::new(other), vec![value]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() -> anyhow::Result<()> {
        let deser = ExpressionToken::parse_stream("1 +2 * ( 3 - 4)")?;
        assert_eq!(
            deser,
            vec![
                ExpressionToken::Literal(1.0),
                ExpressionToken::Add,
                ExpressionToken::Literal(2.0),
                ExpressionToken::Mul,
                ExpressionToken::ParenOpen,
                ExpressionToken::Literal(3.0),
                ExpressionToken::Sub,
                ExpressionToken::Literal(4.0),
                ExpressionToken::ParenClose,
            ]
        );
        return Ok(());
    }
}
