//! Safe deterministic arithmetic expression parsing and evaluation.

use crate::{
    apply_precision, binary_arithmetic, version, ArithmeticOperation, ComputeError,
    ComputeResponse, ComputeResult, Decimal, Number, NumericValue, PrecisionPolicy, ResultMetadata,
    RoundingMode, TraceStep,
};
use std::str::FromStr;

const EXPRESSION_OPERATION_ID: &str = "expression.evaluate";
const MAX_EXPRESSION_BYTES: usize = 16 * 1024;
const MAX_TOKENS: usize = 4096;
const MAX_PARSE_DEPTH: usize = 256;
const MAX_EVAL_DEPTH: usize = 1024;
const I128_MIN_ABS_LITERAL: &str = "170141183460469231731687303715884105728";

/// Parsed arithmetic expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    Number(Number),
    UnaryMinus(Box<Expression>),
    Binary {
        operation: ArithmeticOperation,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

/// Parses a deterministic arithmetic expression into an AST.
pub fn parse_expression(input: &str) -> Result<Expression, ComputeError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, current: 0 };
    let expression = parser.parse_expression()?;
    if parser.peek() != &Token::End {
        return Err(ComputeError::invalid_input(format!(
            "unexpected token at byte {}",
            parser.peek_position()
        )));
    }
    Ok(expression)
}

/// Evaluates an arithmetic expression string.
pub fn evaluate_expression(
    input: &str,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> ComputeResponse {
    match evaluate_expression_result(input, precision, include_trace) {
        Ok((value, trace)) => {
            let result = ComputeResult {
                operation: EXPRESSION_OPERATION_ID.to_owned(),
                value: value.into(),
                metadata: ResultMetadata {
                    engine_version: version().to_owned(),
                    numeric_kind: value.numeric_kind(),
                    precision,
                    deterministic: true,
                },
            };
            ComputeResponse::success(result, include_trace.then_some(trace))
        }
        Err(error) => ComputeResponse::failure(error, None),
    }
}

/// Evaluates a parsed arithmetic expression.
pub fn evaluate_ast(
    expression: &Expression,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<(Number, Vec<TraceStep>), ComputeError> {
    let mut evaluator = Evaluator {
        precision,
        include_trace,
        trace: Vec::new(),
        next_step: 1,
    };
    let value = evaluator.evaluate(expression, 0)?;
    let adjusted = apply_precision(value, precision)?;
    if precision.decimal_places.is_some() {
        evaluator.record(
            "expression.precision",
            vec![value.into()],
            adjusted.into(),
            "deterministic final precision adjustment",
        );
    }
    Ok((adjusted, evaluator.trace))
}

fn evaluate_expression_result(
    input: &str,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<(Number, Vec<TraceStep>), ComputeError> {
    let expression = parse_expression(input)?;
    evaluate_ast(&expression, precision, include_trace)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Number(String),
    Plus,
    Minus,
    Star,
    Slash,
    LeftParen,
    RightParen,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PositionedToken {
    token: Token,
    position: usize,
}

fn tokenize(input: &str) -> Result<Vec<PositionedToken>, ComputeError> {
    if input.len() > MAX_EXPRESSION_BYTES {
        return Err(ComputeError::invalid_input(format!(
            "expression exceeds maximum length of {MAX_EXPRESSION_BYTES} bytes"
        )));
    }

    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' | b'\n' | b'\r' => index += 1,
            b'+' => {
                push_token(&mut tokens, Token::Plus, index)?;
                index += 1;
            }
            b'-' => {
                push_token(&mut tokens, Token::Minus, index)?;
                index += 1;
            }
            b'*' => {
                push_token(&mut tokens, Token::Star, index)?;
                index += 1;
            }
            b'/' => {
                push_token(&mut tokens, Token::Slash, index)?;
                index += 1;
            }
            b'(' => {
                push_token(&mut tokens, Token::LeftParen, index)?;
                index += 1;
            }
            b')' => {
                push_token(&mut tokens, Token::RightParen, index)?;
                index += 1;
            }
            b'0'..=b'9' | b'.' => {
                let start = index;
                index = scan_number(bytes, index)?;
                let literal = input.get(start..index).ok_or_else(|| {
                    ComputeError::invalid_input("invalid UTF-8 boundary in numeric literal")
                })?;
                push_token(&mut tokens, Token::Number(literal.to_owned()), start)?;
            }
            _ => {
                return Err(ComputeError::invalid_input(format!(
                    "invalid token at byte {index}"
                )));
            }
        }
    }

    if tokens.is_empty() {
        return Err(ComputeError::invalid_input("expression is empty"));
    }

    tokens.push(positioned(Token::End, input.len()));
    Ok(tokens)
}

fn positioned(token: Token, position: usize) -> PositionedToken {
    PositionedToken { token, position }
}

fn push_token(
    tokens: &mut Vec<PositionedToken>,
    token: Token,
    position: usize,
) -> Result<(), ComputeError> {
    if tokens.len() >= MAX_TOKENS {
        return Err(ComputeError::invalid_input(format!(
            "expression exceeds maximum token count of {MAX_TOKENS}"
        )));
    }
    tokens.push(positioned(token, position));
    Ok(())
}

fn scan_number(bytes: &[u8], start: usize) -> Result<usize, ComputeError> {
    let mut index = start;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }

    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        let fraction_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if fraction_start == index {
            return Err(ComputeError::invalid_input(format!(
                "decimal literal at byte {start} requires digits after decimal point"
            )));
        }
    }

    if index == start {
        return Err(ComputeError::invalid_input(format!(
            "numeric literal expected at byte {start}"
        )));
    }

    Ok(index)
}

fn parse_number(literal: &str) -> Result<Number, ComputeError> {
    if literal.contains('.') {
        Decimal::from_str(literal).map(Number::Decimal)
    } else {
        literal
            .parse::<i128>()
            .map(Number::Integer)
            .map_err(|_| ComputeError::invalid_input(format!("invalid integer literal: {literal}")))
    }
}

struct Parser {
    tokens: Vec<PositionedToken>,
    current: usize,
}

impl Parser {
    fn parse_expression(&mut self) -> Result<Expression, ComputeError> {
        self.parse_additive(0)
    }

    fn parse_additive(&mut self, depth: usize) -> Result<Expression, ComputeError> {
        self.check_parse_depth(depth)?;
        let mut expression = self.parse_multiplicative(depth + 1)?;
        loop {
            let operation = match self.peek() {
                Token::Plus => ArithmeticOperation::Add,
                Token::Minus => ArithmeticOperation::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative(depth + 1)?;
            expression = Expression::Binary {
                operation,
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_multiplicative(&mut self, depth: usize) -> Result<Expression, ComputeError> {
        self.check_parse_depth(depth)?;
        let mut expression = self.parse_unary(depth + 1)?;
        loop {
            let operation = match self.peek() {
                Token::Star => ArithmeticOperation::Multiply,
                Token::Slash => ArithmeticOperation::Divide,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary(depth + 1)?;
            expression = Expression::Binary {
                operation,
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_unary(&mut self, depth: usize) -> Result<Expression, ComputeError> {
        self.check_parse_depth(depth)?;
        if self.peek() == &Token::Minus {
            self.advance();
            if let Token::Number(literal) = self.peek().clone() {
                if literal == I128_MIN_ABS_LITERAL {
                    self.advance();
                    return Ok(Expression::Number(Number::Integer(i128::MIN)));
                }
            }
            return Ok(Expression::UnaryMinus(Box::new(
                self.parse_unary(depth + 1)?,
            )));
        }
        self.parse_primary(depth + 1)
    }

    fn parse_primary(&mut self, depth: usize) -> Result<Expression, ComputeError> {
        self.check_parse_depth(depth)?;
        match self.peek().clone() {
            Token::Number(literal) => {
                self.advance();
                Ok(Expression::Number(parse_number(&literal)?))
            }
            Token::LeftParen => {
                self.advance();
                let expression = self.parse_additive(depth + 1)?;
                if self.peek() != &Token::RightParen {
                    return Err(ComputeError::invalid_input(format!(
                        "expected ')' at byte {}",
                        self.peek_position()
                    )));
                }
                self.advance();
                Ok(expression)
            }
            Token::End => Err(ComputeError::invalid_input("expression is incomplete")),
            _ => Err(ComputeError::invalid_input(format!(
                "expected number or '(' at byte {}",
                self.peek_position()
            ))),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current].token
    }

    fn peek_position(&self) -> usize {
        self.tokens[self.current].position
    }

    fn advance(&mut self) {
        if self.current + 1 < self.tokens.len() {
            self.current += 1;
        }
    }

    fn check_parse_depth(&self, depth: usize) -> Result<(), ComputeError> {
        if depth > MAX_PARSE_DEPTH {
            return Err(ComputeError::invalid_input(format!(
                "expression nesting exceeds maximum depth of {MAX_PARSE_DEPTH}"
            )));
        }
        Ok(())
    }
}

struct Evaluator {
    precision: PrecisionPolicy,
    include_trace: bool,
    trace: Vec<TraceStep>,
    next_step: u32,
}

impl Evaluator {
    fn evaluate(&mut self, expression: &Expression, depth: usize) -> Result<Number, ComputeError> {
        if depth > MAX_EVAL_DEPTH {
            return Err(ComputeError::invalid_input(format!(
                "expression evaluation exceeds maximum depth of {MAX_EVAL_DEPTH}"
            )));
        }

        match expression {
            Expression::Number(number) => Ok(*number),
            Expression::UnaryMinus(inner) => {
                let input = self.evaluate(inner, depth + 1)?;
                let output = binary_arithmetic(
                    ArithmeticOperation::Subtract,
                    Number::Integer(0),
                    input,
                    PrecisionPolicy::default(),
                )?;
                self.record(
                    "expression.negate",
                    vec![input.into()],
                    output.into(),
                    "deterministic unary minus",
                );
                Ok(output)
            }
            Expression::Binary {
                operation,
                left,
                right,
            } => {
                let left = self.evaluate(left, depth + 1)?;
                let right = self.evaluate(right, depth + 1)?;
                let output = self.evaluate_binary(*operation, left, right)?;
                self.record(
                    operation.operation_id(),
                    vec![left.into(), right.into()],
                    output.into(),
                    "deterministic expression arithmetic",
                );
                Ok(output)
            }
        }
    }

    fn evaluate_binary(
        &self,
        operation: ArithmeticOperation,
        left: Number,
        right: Number,
    ) -> Result<Number, ComputeError> {
        if operation != ArithmeticOperation::Divide {
            return binary_arithmetic(operation, left, right, PrecisionPolicy::default());
        }

        match binary_arithmetic(operation, left, right, PrecisionPolicy::default()) {
            Ok(value) => Ok(value),
            Err(error) if can_round_repeating_division(&error, self.precision) => {
                binary_arithmetic(operation, left, right, self.precision)
            }
            Err(error) => Err(error),
        }
    }

    fn record(
        &mut self,
        operation: impl Into<String>,
        inputs: Vec<NumericValue>,
        output: NumericValue,
        note: impl Into<String>,
    ) {
        if !self.include_trace {
            return;
        }

        self.trace.push(TraceStep {
            step: self.next_step,
            operation: operation.into(),
            inputs,
            output: Some(output),
            note: note.into(),
        });
        self.next_step += 1;
    }
}

fn can_round_repeating_division(error: &ComputeError, precision: PrecisionPolicy) -> bool {
    error.is_repeating_decimal_expansion()
        && precision.decimal_places.is_some()
        && precision.rounding != RoundingMode::Exact
}

#[cfg(test)]
mod tests {
    use super::{evaluate_expression, parse_expression, Expression};
    use crate::{ComputeError, ErrorCode, NumericValue, PrecisionPolicy, RoundingMode};

    fn result_value(input: &str) -> Option<NumericValue> {
        evaluate_expression(input, PrecisionPolicy::default(), false)
            .result
            .map(|result| result.value)
    }

    fn exact_scale(decimal_places: u32) -> PrecisionPolicy {
        PrecisionPolicy {
            decimal_places: Some(decimal_places),
            rounding: RoundingMode::Exact,
        }
    }

    #[test]
    fn honors_operator_precedence() {
        assert_eq!(
            result_value("2 + 3 * 4"),
            Some(NumericValue::Integer {
                value: "14".to_owned()
            })
        );
    }

    #[test]
    fn honors_parentheses() {
        assert_eq!(
            result_value("(2 + 3) * 4"),
            Some(NumericValue::Integer {
                value: "20".to_owned()
            })
        );
    }

    #[test]
    fn parses_unary_minus() {
        assert_eq!(
            result_value("-2 * -(3 + 4)"),
            Some(NumericValue::Integer {
                value: "14".to_owned()
            })
        );
    }

    #[test]
    fn evaluates_decimal_arithmetic() {
        assert_eq!(
            result_value("1.20 + .30"),
            Some(NumericValue::Decimal {
                value: "1.5".to_owned(),
                scale: 1
            })
        );
    }

    #[test]
    fn applies_division_rounding_policy() {
        let response = evaluate_expression(
            "1 / 8",
            PrecisionPolicy {
                decimal_places: Some(2),
                rounding: RoundingMode::HalfAwayFromZero,
            },
            false,
        );

        assert_eq!(
            response.result.map(|result| result.value),
            Some(NumericValue::Decimal {
                value: "0.13".to_owned(),
                scale: 2
            })
        );
    }

    #[test]
    fn applies_output_precision_only_after_decimal_cancellation() {
        let response = evaluate_expression("1.234 - 1.234", exact_scale(2), false);

        assert_eq!(
            response.result.map(|result| result.value),
            Some(NumericValue::Decimal {
                value: "0.00".to_owned(),
                scale: 2
            })
        );
    }

    #[test]
    fn avoids_rejecting_valid_final_precision_from_intermediate_scale() {
        let response = evaluate_expression("1.235 - 0.005", exact_scale(2), false);

        assert_eq!(
            response.result.map(|result| result.value),
            Some(NumericValue::Decimal {
                value: "1.23".to_owned(),
                scale: 2
            })
        );
    }

    #[test]
    fn preserves_terminating_division_precision_for_later_cancellation() {
        let response = evaluate_expression("1 / 8 * 8", exact_scale(2), false);

        assert_eq!(
            response.result.map(|result| result.value),
            Some(NumericValue::Decimal {
                value: "1.00".to_owned(),
                scale: 2
            })
        );
    }

    #[test]
    fn rejects_invalid_tokens() {
        let response = evaluate_expression("1 + two", PrecisionPolicy::default(), false);

        assert!(!response.ok);
        assert_eq!(
            response.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
    }

    #[test]
    fn rejects_malformed_syntax() {
        let response = evaluate_expression("1 + * 2", PrecisionPolicy::default(), false);

        assert!(!response.ok);
        assert_eq!(
            response.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
    }

    #[test]
    fn rejects_tokenization_edge_cases() {
        for input in ["1..2", ".", "1.", "1e2", "1 2", "", "   \t\n"] {
            let response = evaluate_expression(input, PrecisionPolicy::default(), false);

            assert!(!response.ok, "{input:?} should be rejected");
            assert_eq!(
                response.error.map(|error| error.code),
                Some(ErrorCode::InvalidInput),
                "{input:?} should return invalid input"
            );
        }
    }

    #[test]
    fn rejects_excessive_parenthesis_depth() {
        let expression = format!("{}1{}", "(".repeat(300), ")".repeat(300));
        let response = evaluate_expression(&expression, PrecisionPolicy::default(), false);

        assert!(!response.ok);
        let error = response.error.unwrap_or_else(|| {
            ComputeError::invalid_input("expected depth error for parenthesis chain")
        });
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error
            .message
            .contains("expression nesting exceeds maximum depth"));
    }

    #[test]
    fn rejects_excessive_unary_minus_depth() {
        let expression = format!("{}1", "-".repeat(300));
        let response = evaluate_expression(&expression, PrecisionPolicy::default(), false);

        assert!(!response.ok);
        let error = response.error.unwrap_or_else(|| {
            ComputeError::invalid_input("expected depth error for unary minus chain")
        });
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error
            .message
            .contains("expression nesting exceeds maximum depth"));
    }

    #[test]
    fn supports_i128_min_as_unary_minus_literal() {
        assert_eq!(
            result_value("-170141183460469231731687303715884105728"),
            Some(NumericValue::Integer {
                value: i128::MIN.to_string()
            })
        );
    }

    #[test]
    fn produces_deterministic_trace() {
        let first = evaluate_expression("2 * (3 + 4)", PrecisionPolicy::default(), true);
        let second = evaluate_expression("2 * (3 + 4)", PrecisionPolicy::default(), true);

        assert_eq!(first.trace, second.trace);
        let trace = first.trace.unwrap_or_default();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].step, 1);
        assert_eq!(trace[0].operation, "arithmetic.add");
        assert_eq!(trace[1].step, 2);
        assert_eq!(trace[1].operation, "arithmetic.multiply");
    }

    #[test]
    fn traces_final_precision_adjustment() {
        let response = evaluate_expression("1.234 - 1.234", exact_scale(2), true);
        let trace = response.trace.unwrap_or_default();

        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].operation, "arithmetic.subtract");
        assert_eq!(trace[1].step, 2);
        assert_eq!(trace[1].operation, "expression.precision");
        assert_eq!(
            trace[1].inputs,
            vec![NumericValue::Decimal {
                value: "0".to_owned(),
                scale: 0
            }]
        );
        assert_eq!(
            trace[1].output,
            Some(NumericValue::Decimal {
                value: "0.00".to_owned(),
                scale: 2
            })
        );
    }

    #[test]
    fn rejects_precision_loss_without_rounding_policy() {
        let response = evaluate_expression(
            "1 / 8",
            PrecisionPolicy {
                decimal_places: Some(2),
                rounding: RoundingMode::Exact,
            },
            false,
        );

        assert!(!response.ok);
        assert_eq!(
            response.error.map(|error| error.code),
            Some(ErrorCode::PrecisionIssue)
        );
    }

    #[test]
    fn parses_ast_without_evaluating() {
        assert!(matches!(
            parse_expression("1 + 2"),
            Ok(Expression::Binary { .. })
        ));
    }
}
