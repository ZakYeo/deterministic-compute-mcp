//! Deterministic computation core.
//!
//! This crate provides schema-friendly request/response models and trusted
//! deterministic numeric primitives. Higher-level workstreams own expression
//! parsing, unit conversion, finance, verification, and test generation.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

pub mod expression;
mod precision;

use precision::MAX_DECIMAL_SCALE;

/// Current foundation API status for downstream scaffolds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngineStatus {
    /// Repository foundation exists, with compute-core primitives available.
    FoundationOnly,
}

/// Generic compute request suitable for CLI and MCP wrappers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeRequest {
    /// Stable operation identifier, such as `arithmetic.add`.
    pub operation: String,
    /// Operation-specific input payload.
    pub input: Value,
    /// Optional decimal precision and rounding policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<PrecisionPolicy>,
    /// Whether the caller requests deterministic step metadata.
    #[serde(default)]
    pub trace: bool,
}

/// Generic compute response suitable for CLI and MCP wrappers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ComputeResult>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<TraceStep>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ComputeError>,
    pub version: String,
}

impl ComputeResponse {
    /// Builds a successful response with stable metadata.
    #[must_use]
    pub fn success(result: ComputeResult, trace: Option<Vec<TraceStep>>) -> Self {
        Self {
            ok: true,
            result: Some(result),
            diagnostics: Vec::new(),
            trace,
            error: None,
            version: version().to_owned(),
        }
    }

    /// Builds an error response with stable metadata.
    #[must_use]
    pub fn failure(error: ComputeError, trace: Option<Vec<TraceStep>>) -> Self {
        Self {
            ok: false,
            result: None,
            diagnostics: Vec::new(),
            trace,
            error: Some(error),
            version: version().to_owned(),
        }
    }
}

/// Deterministic operation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeResult {
    pub operation: String,
    pub value: NumericValue,
    pub metadata: ResultMetadata,
}

/// Stable metadata emitted with deterministic results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultMetadata {
    pub engine_version: String,
    pub numeric_kind: NumericKind,
    pub precision: PrecisionPolicy,
    pub deterministic: bool,
}

/// Diagnostic severity for non-fatal messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
}

/// Structured diagnostic for warnings, assumptions, and validation notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Deterministic trace step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceStep {
    pub step: u32,
    pub operation: String,
    pub inputs: Vec<NumericValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<NumericValue>,
    pub note: String,
}

/// Structured compute error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ComputeError {
    pub(crate) fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidInput,
            message: message.into(),
            detail: None,
        }
    }

    pub(crate) fn division_by_zero() -> Self {
        Self {
            code: ErrorCode::DivisionByZero,
            message: "division by zero".to_owned(),
            detail: None,
        }
    }

    pub(crate) fn precision_issue(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::PrecisionIssue,
            message: message.into(),
            detail: None,
        }
    }

    pub(crate) fn repeating_decimal_expansion() -> Self {
        Self::precision_issue(REPEATING_DECIMAL_EXPANSION_MESSAGE)
    }

    pub(crate) fn is_repeating_decimal_expansion(&self) -> bool {
        self.code == ErrorCode::PrecisionIssue
            && self.message == REPEATING_DECIMAL_EXPANSION_MESSAGE
    }

    pub(crate) fn overflow(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Overflow,
            message: message.into(),
            detail: None,
        }
    }
}

const REPEATING_DECIMAL_EXPANSION_MESSAGE: &str =
    "division result has a repeating decimal expansion";

impl fmt::Display for ComputeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ComputeError {}

/// Stable error codes for machine-readable failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    InvalidInput,
    DivisionByZero,
    PrecisionIssue,
    Overflow,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::InvalidInput => "invalid-input",
            Self::DivisionByZero => "division-by-zero",
            Self::PrecisionIssue => "precision-issue",
            Self::Overflow => "overflow",
        };
        formatter.write_str(code)
    }
}

/// Explicit precision and rounding policy for decimal operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrecisionPolicy {
    /// Number of fractional decimal places required for the output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimal_places: Option<u32>,
    pub rounding: RoundingMode,
}

impl Default for PrecisionPolicy {
    fn default() -> Self {
        Self {
            decimal_places: None,
            rounding: RoundingMode::Exact,
        }
    }
}

/// Supported rounding modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoundingMode {
    /// Reject outputs that cannot be represented exactly at the requested scale.
    Exact,
    /// Drop excess fractional places toward zero.
    Truncate,
    /// Round halves away from zero.
    HalfAwayFromZero,
}

/// Numeric output kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NumericKind {
    Integer,
    Decimal,
}

/// JSON-safe deterministic numeric value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum NumericValue {
    Integer { value: String },
    Decimal { value: String, scale: u32 },
}

impl NumericValue {
    /// Parses a JSON-safe numeric value into a deterministic number.
    pub fn parse_number(&self) -> Result<Number, ComputeError> {
        match self {
            Self::Integer { value } => {
                let parsed = value.parse::<i128>().map_err(|_| {
                    ComputeError::invalid_input(format!("invalid integer literal: {value}"))
                })?;
                Ok(Number::Integer(parsed))
            }
            Self::Decimal { value, scale } => {
                let parsed = Decimal::parse_with_explicit_scale(value, *scale)?;
                Ok(Number::Decimal(parsed))
            }
        }
    }
}

impl<'de> Deserialize<'de> for NumericValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "kebab-case")]
        enum WireNumericValue {
            Integer { value: String },
            Decimal { value: String, scale: u32 },
        }

        match WireNumericValue::deserialize(deserializer)? {
            WireNumericValue::Integer { value } => value
                .parse::<i128>()
                .map(|_| Self::Integer { value })
                .map_err(|_| de::Error::custom("invalid integer literal")),
            WireNumericValue::Decimal { value, scale } => {
                Decimal::parse_with_explicit_scale(&value, scale)
                    .map_err(de::Error::custom)
                    .map(|_| Self::Decimal { value, scale })
            }
        }
    }
}

impl From<Number> for NumericValue {
    fn from(value: Number) -> Self {
        match value {
            Number::Integer(value) => Self::Integer {
                value: value.to_string(),
            },
            Number::Decimal(value) => Self::Decimal {
                value: value.to_string(),
                scale: value.scale,
            },
        }
    }
}

/// Deterministic number representation used by arithmetic primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Number {
    Integer(i128),
    Decimal(Decimal),
}

impl Number {
    #[must_use]
    pub fn numeric_kind(self) -> NumericKind {
        match self {
            Self::Integer(_) => NumericKind::Integer,
            Self::Decimal(_) => NumericKind::Decimal,
        }
    }

    fn into_decimal(self) -> Decimal {
        match self {
            Self::Integer(value) => Decimal::new_unchecked(value, 0),
            Self::Decimal(value) => value,
        }
    }
}

impl From<i128> for Number {
    fn from(value: i128) -> Self {
        Self::Integer(value)
    }
}

impl From<Decimal> for Number {
    fn from(value: Decimal) -> Self {
        Self::Decimal(value)
    }
}

/// Fixed-scale base-10 decimal represented as `coefficient * 10^-scale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal {
    coefficient: i128,
    scale: u32,
}

impl Decimal {
    /// Creates a normalized decimal from a coefficient and scale.
    pub fn new(coefficient: i128, scale: u32) -> Result<Self, ComputeError> {
        Self::validate_scale(scale)?;
        Ok(Self::new_unchecked(coefficient, scale).normalized())
    }

    /// Creates a decimal with an explicit scale, preserving trailing zeroes.
    pub fn with_scale(coefficient: i128, scale: u32) -> Result<Self, ComputeError> {
        Self::validate_scale(scale)?;
        Ok(Self::new_unchecked(coefficient, scale))
    }

    #[must_use]
    pub fn coefficient(self) -> i128 {
        self.coefficient
    }

    #[must_use]
    pub fn scale(self) -> u32 {
        self.scale
    }

    fn normalized(mut self) -> Self {
        while self.scale > 0 && self.coefficient % 10 == 0 {
            self.coefficient /= 10;
            self.scale -= 1;
        }
        self
    }

    fn new_unchecked(coefficient: i128, scale: u32) -> Self {
        Self { coefficient, scale }
    }

    fn validate_scale(scale: u32) -> Result<(), ComputeError> {
        if scale > MAX_DECIMAL_SCALE {
            return Err(ComputeError::precision_issue(format!(
                "decimal scale {scale} exceeds maximum {MAX_DECIMAL_SCALE}"
            )));
        }
        Ok(())
    }

    fn parse_with_explicit_scale(input: &str, expected_scale: u32) -> Result<Self, ComputeError> {
        let decimal = Self::parse_literal(input, false)?;
        if decimal.scale != expected_scale {
            return Err(ComputeError::invalid_input(format!(
                "decimal value scale {} does not match serialized scale {expected_scale}",
                decimal.scale
            )));
        }
        Ok(decimal)
    }

    fn parse_literal(input: &str, normalize: bool) -> Result<Self, ComputeError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ComputeError::invalid_input("decimal literal is empty"));
        }

        let (negative, unsigned) = match trimmed.as_bytes()[0] {
            b'-' => (true, &trimmed[1..]),
            b'+' => (false, &trimmed[1..]),
            _ => (false, trimmed),
        };

        if unsigned.is_empty() {
            return Err(ComputeError::invalid_input(format!(
                "invalid decimal literal: {input}"
            )));
        }

        let parts = unsigned.split('.').collect::<Vec<_>>();
        if parts.len() > 2 {
            return Err(ComputeError::invalid_input(format!(
                "invalid decimal literal: {input}"
            )));
        }

        let whole = parts[0];
        let fraction = parts.get(1).copied().unwrap_or_default();
        if whole.is_empty() && fraction.is_empty() {
            return Err(ComputeError::invalid_input(format!(
                "invalid decimal literal: {input}"
            )));
        }
        if !whole.chars().all(|character| character.is_ascii_digit())
            || !fraction.chars().all(|character| character.is_ascii_digit())
        {
            return Err(ComputeError::invalid_input(format!(
                "invalid decimal literal: {input}"
            )));
        }

        let scale = u32::try_from(fraction.len())
            .map_err(|_| ComputeError::precision_issue("decimal scale is too large"))?;
        Self::validate_scale(scale)?;

        let unsigned_digits = format!("{whole}{fraction}");
        let signed_digits = if negative {
            format!("-{unsigned_digits}")
        } else {
            unsigned_digits
        };
        let coefficient = signed_digits.parse::<i128>().map_err(|_| {
            ComputeError::invalid_input(format!("decimal literal is too large: {input}"))
        })?;

        if normalize {
            Self::new(coefficient, scale)
        } else {
            Self::with_scale(coefficient, scale)
        }
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return write!(formatter, "{}", self.coefficient);
        }

        let negative = self.coefficient.is_negative();
        let digits = self.coefficient.unsigned_abs().to_string();
        let scale = self.scale as usize;

        if negative {
            write!(formatter, "-")?;
        }

        match digits.len().cmp(&scale) {
            Ordering::Greater => {
                let split = digits.len() - scale;
                write!(formatter, "{}.{}", &digits[..split], &digits[split..])
            }
            Ordering::Equal => write!(formatter, "0.{digits}"),
            Ordering::Less => {
                write!(formatter, "0.")?;
                for _ in 0..(scale - digits.len()) {
                    write!(formatter, "0")?;
                }
                write!(formatter, "{digits}")
            }
        }
    }
}

impl FromStr for Decimal {
    type Err = ComputeError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse_literal(input, false)
    }
}

/// Basic arithmetic operations owned by compute-core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArithmeticOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl ArithmeticOperation {
    #[must_use]
    pub fn operation_id(self) -> &'static str {
        match self {
            Self::Add => "arithmetic.add",
            Self::Subtract => "arithmetic.subtract",
            Self::Multiply => "arithmetic.multiply",
            Self::Divide => "arithmetic.divide",
        }
    }
}

/// Evaluates a binary arithmetic operation with deterministic metadata.
pub fn compute_binary(
    operation: ArithmeticOperation,
    left: Number,
    right: Number,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> ComputeResponse {
    match binary_arithmetic(operation, left, right, precision) {
        Ok(value) => {
            let result = ComputeResult {
                operation: operation.operation_id().to_owned(),
                value: value.into(),
                metadata: ResultMetadata {
                    engine_version: version().to_owned(),
                    numeric_kind: value.numeric_kind(),
                    precision,
                    deterministic: true,
                },
            };
            let trace = include_trace.then(|| {
                vec![TraceStep {
                    step: 1,
                    operation: operation.operation_id().to_owned(),
                    inputs: vec![left.into(), right.into()],
                    output: Some(value.into()),
                    note: "deterministic binary arithmetic".to_owned(),
                }]
            });
            ComputeResponse::success(result, trace)
        }
        Err(error) => ComputeResponse::failure(error, None),
    }
}

/// Evaluates a binary arithmetic operation.
pub fn binary_arithmetic(
    operation: ArithmeticOperation,
    left: Number,
    right: Number,
    precision: PrecisionPolicy,
) -> Result<Number, ComputeError> {
    let result = match operation {
        ArithmeticOperation::Add => add(left, right)?,
        ArithmeticOperation::Subtract => subtract(left, right)?,
        ArithmeticOperation::Multiply => multiply(left, right)?,
        ArithmeticOperation::Divide => divide(left, right, precision)?,
    };
    apply_precision(result, precision)
}

/// Adds two deterministic numbers.
pub fn add(left: Number, right: Number) -> Result<Number, ComputeError> {
    match (left, right) {
        (Number::Integer(left), Number::Integer(right)) => left
            .checked_add(right)
            .map(Number::Integer)
            .ok_or_else(|| ComputeError::overflow("integer addition overflow")),
        _ => add_decimal(left.into_decimal(), right.into_decimal()).map(Number::Decimal),
    }
}

/// Subtracts two deterministic numbers.
pub fn subtract(left: Number, right: Number) -> Result<Number, ComputeError> {
    match (left, right) {
        (Number::Integer(left), Number::Integer(right)) => left
            .checked_sub(right)
            .map(Number::Integer)
            .ok_or_else(|| ComputeError::overflow("integer subtraction overflow")),
        _ => subtract_decimal(left.into_decimal(), right.into_decimal()).map(Number::Decimal),
    }
}

/// Multiplies two deterministic numbers.
pub fn multiply(left: Number, right: Number) -> Result<Number, ComputeError> {
    match (left, right) {
        (Number::Integer(left), Number::Integer(right)) => left
            .checked_mul(right)
            .map(Number::Integer)
            .ok_or_else(|| ComputeError::overflow("integer multiplication overflow")),
        _ => multiply_decimal(left.into_decimal(), right.into_decimal()).map(Number::Decimal),
    }
}

/// Divides two deterministic numbers.
pub fn divide(
    left: Number,
    right: Number,
    precision: PrecisionPolicy,
) -> Result<Number, ComputeError> {
    let left = left.into_decimal();
    let right = right.into_decimal();
    if right.coefficient == 0 {
        return Err(ComputeError::division_by_zero());
    }

    divide_decimal(left, right, precision).map(Number::Decimal)
}

fn add_decimal(left: Decimal, right: Decimal) -> Result<Decimal, ComputeError> {
    let scale = left.scale.max(right.scale);
    let left_coefficient = align_coefficient(left, scale)?;
    let right_coefficient = align_coefficient(right, scale)?;
    left_coefficient
        .checked_add(right_coefficient)
        .ok_or_else(|| ComputeError::overflow("decimal addition overflow"))
        .and_then(|coefficient| Decimal::new(coefficient, scale))
}

fn subtract_decimal(left: Decimal, right: Decimal) -> Result<Decimal, ComputeError> {
    let scale = left.scale.max(right.scale);
    let left_coefficient = align_coefficient(left, scale)?;
    let right_coefficient = align_coefficient(right, scale)?;
    left_coefficient
        .checked_sub(right_coefficient)
        .ok_or_else(|| ComputeError::overflow("decimal subtraction overflow"))
        .and_then(|coefficient| Decimal::new(coefficient, scale))
}

fn multiply_decimal(left: Decimal, right: Decimal) -> Result<Decimal, ComputeError> {
    let scale = left
        .scale
        .checked_add(right.scale)
        .ok_or_else(|| ComputeError::precision_issue("decimal multiplication scale overflow"))?;
    if scale > MAX_DECIMAL_SCALE {
        return Err(ComputeError::precision_issue(format!(
            "decimal scale {scale} exceeds maximum {MAX_DECIMAL_SCALE}"
        )));
    }
    left.coefficient
        .checked_mul(right.coefficient)
        .ok_or_else(|| ComputeError::overflow("decimal multiplication overflow"))
        .and_then(|coefficient| Decimal::new(coefficient, scale))
}

fn divide_decimal(
    left: Decimal,
    right: Decimal,
    precision: PrecisionPolicy,
) -> Result<Decimal, ComputeError> {
    let (numerator, denominator) = decimal_division_fraction(left, right)?;

    if let Some(decimal_places) = precision.decimal_places {
        let scaled_numerator = numerator
            .checked_mul(pow10(decimal_places)?)
            .ok_or_else(|| ComputeError::overflow("decimal division scaling overflow"))?;
        let quotient = divide_and_round(scaled_numerator, denominator, precision.rounding)?;
        return Decimal::with_scale(quotient, decimal_places);
    }

    let extra_scale = terminating_scale(denominator)?;
    let scaled_numerator = numerator
        .checked_mul(pow10(extra_scale)?)
        .ok_or_else(|| ComputeError::overflow("decimal division scaling overflow"))?;
    let quotient = divide_exact(scaled_numerator, denominator)?;
    Decimal::new(quotient, extra_scale)
}

fn decimal_division_fraction(left: Decimal, right: Decimal) -> Result<(i128, i128), ComputeError> {
    let (mut numerator, mut denominator) = reduce_fraction(left.coefficient, right.coefficient)?;
    let scale_shift = i64::from(right.scale) - i64::from(left.scale);

    if scale_shift > 0 {
        let shift = u32::try_from(scale_shift)
            .map_err(|_| ComputeError::precision_issue("decimal scale shift overflow"))?;
        (numerator, denominator) = apply_decimal_shift_to_numerator(numerator, denominator, shift)?;
    } else if scale_shift < 0 {
        let shift = u32::try_from(-scale_shift)
            .map_err(|_| ComputeError::precision_issue("decimal scale shift overflow"))?;
        (numerator, denominator) =
            apply_decimal_shift_to_denominator(numerator, denominator, shift)?;
    }

    reduce_fraction(numerator, denominator)
}

fn apply_decimal_shift_to_numerator(
    mut numerator: i128,
    mut denominator: i128,
    shift: u32,
) -> Result<(i128, i128), ComputeError> {
    let mut twos = shift;
    let mut fives = shift;

    while twos > 0 && denominator % 2 == 0 {
        denominator = checked_div(denominator, 2)?;
        twos -= 1;
    }
    while fives > 0 && denominator % 5 == 0 {
        denominator = checked_div(denominator, 5)?;
        fives -= 1;
    }

    numerator =
        multiply_by_repeated_factor(numerator, 2, twos, "decimal division scaling overflow")?;
    numerator =
        multiply_by_repeated_factor(numerator, 5, fives, "decimal division scaling overflow")?;
    Ok((numerator, denominator))
}

fn apply_decimal_shift_to_denominator(
    mut numerator: i128,
    mut denominator: i128,
    shift: u32,
) -> Result<(i128, i128), ComputeError> {
    let mut twos = shift;
    let mut fives = shift;

    while twos > 0 && numerator % 2 == 0 {
        numerator = checked_div(numerator, 2)?;
        twos -= 1;
    }
    while fives > 0 && numerator % 5 == 0 {
        numerator = checked_div(numerator, 5)?;
        fives -= 1;
    }

    denominator =
        multiply_by_repeated_factor(denominator, 2, twos, "decimal division scaling overflow")?;
    denominator =
        multiply_by_repeated_factor(denominator, 5, fives, "decimal division scaling overflow")?;
    Ok((numerator, denominator))
}

fn multiply_by_repeated_factor(
    mut value: i128,
    factor: i128,
    count: u32,
    overflow_message: &'static str,
) -> Result<i128, ComputeError> {
    for _ in 0..count {
        value = value
            .checked_mul(factor)
            .ok_or_else(|| ComputeError::overflow(overflow_message))?;
    }
    Ok(value)
}

pub(crate) fn apply_precision(
    value: Number,
    precision: PrecisionPolicy,
) -> Result<Number, ComputeError> {
    let Some(decimal_places) = precision.decimal_places else {
        return Ok(value);
    };

    if decimal_places > MAX_DECIMAL_SCALE {
        return Err(ComputeError::precision_issue(format!(
            "decimal places {decimal_places} exceeds maximum {MAX_DECIMAL_SCALE}"
        )));
    }

    let decimal = value.into_decimal();
    if decimal.scale <= decimal_places {
        let scale_delta = decimal_places - decimal.scale;
        let coefficient = decimal
            .coefficient
            .checked_mul(pow10(scale_delta)?)
            .ok_or_else(|| ComputeError::overflow("decimal precision scaling overflow"))?;
        return Decimal::with_scale(coefficient, decimal_places).map(Number::Decimal);
    }

    let scale_delta = decimal.scale - decimal_places;
    let divisor = pow10(scale_delta)?;
    let rounded = divide_and_round(decimal.coefficient, divisor, precision.rounding)?;
    Decimal::with_scale(rounded, decimal_places).map(Number::Decimal)
}

fn divide_and_round(
    numerator: i128,
    denominator: i128,
    rounding: RoundingMode,
) -> Result<i128, ComputeError> {
    let quotient = checked_div(numerator, denominator)?;
    let remainder = checked_rem(numerator, denominator)?;
    if remainder == 0 {
        return Ok(quotient);
    }

    match rounding {
        RoundingMode::Exact => Err(ComputeError::precision_issue(
            "result cannot be represented exactly with requested precision",
        )),
        RoundingMode::Truncate => Ok(quotient),
        RoundingMode::HalfAwayFromZero => {
            let abs_remainder = remainder.unsigned_abs();
            let abs_denominator = denominator.unsigned_abs();
            let should_increment = abs_remainder
                .checked_mul(2)
                .is_some_and(|doubled| doubled >= abs_denominator);
            if should_increment {
                let sign = if (numerator < 0) == (denominator < 0) {
                    1
                } else {
                    -1
                };
                quotient
                    .checked_add(sign)
                    .ok_or_else(|| ComputeError::overflow("rounded quotient overflow"))
            } else {
                Ok(quotient)
            }
        }
    }
}

fn divide_exact(numerator: i128, denominator: i128) -> Result<i128, ComputeError> {
    let quotient = checked_div(numerator, denominator)?;
    let remainder = checked_rem(numerator, denominator)?;
    if remainder == 0 {
        Ok(quotient)
    } else {
        Err(ComputeError::repeating_decimal_expansion())
    }
}

fn checked_div(numerator: i128, denominator: i128) -> Result<i128, ComputeError> {
    numerator
        .checked_div(denominator)
        .ok_or_else(|| ComputeError::overflow("integer division overflow"))
}

fn checked_rem(numerator: i128, denominator: i128) -> Result<i128, ComputeError> {
    numerator
        .checked_rem(denominator)
        .ok_or_else(|| ComputeError::overflow("integer remainder overflow"))
}

fn reduce_fraction(numerator: i128, denominator: i128) -> Result<(i128, i128), ComputeError> {
    let greatest_common_divisor = gcd_u128(numerator.unsigned_abs(), denominator.unsigned_abs());
    if greatest_common_divisor == 0 {
        return Ok((numerator, denominator));
    }

    if greatest_common_divisor > i128::MAX as u128 {
        return reduce_fraction_by_i128_min_boundary(numerator, denominator);
    }

    let divisor = i128::try_from(greatest_common_divisor)
        .map_err(|_| ComputeError::overflow("fraction reduction divisor overflow"))?;
    let reduced_numerator = checked_div(numerator, divisor)?;
    let reduced_denominator = checked_div(denominator, divisor)?;

    normalize_fraction_sign(reduced_numerator, reduced_denominator)
}

fn reduce_fraction_by_i128_min_boundary(
    numerator: i128,
    denominator: i128,
) -> Result<(i128, i128), ComputeError> {
    let boundary = i128::MIN.unsigned_abs();
    if numerator.unsigned_abs() != boundary || denominator.unsigned_abs() != boundary {
        return Err(ComputeError::overflow(
            "fraction reduction divisor overflow",
        ));
    }

    let reduced_numerator = if numerator.is_negative() { -1 } else { 1 };
    let reduced_denominator = if denominator.is_negative() { -1 } else { 1 };
    normalize_fraction_sign(reduced_numerator, reduced_denominator)
}

fn normalize_fraction_sign(
    numerator: i128,
    denominator: i128,
) -> Result<(i128, i128), ComputeError> {
    if denominator < 0 {
        return Ok((
            numerator
                .checked_neg()
                .ok_or_else(|| ComputeError::overflow("fraction numerator sign overflow"))?,
            denominator
                .checked_neg()
                .ok_or_else(|| ComputeError::overflow("fraction denominator sign overflow"))?,
        ));
    }

    Ok((numerator, denominator))
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn terminating_scale(denominator: i128) -> Result<u32, ComputeError> {
    let mut remaining = denominator;
    let mut twos = 0;
    let mut fives = 0;

    while remaining % 2 == 0 {
        remaining /= 2;
        twos += 1;
    }
    while remaining % 5 == 0 {
        remaining /= 5;
        fives += 1;
    }

    if remaining.unsigned_abs() != 1 {
        return Err(ComputeError::repeating_decimal_expansion());
    }

    Ok(twos.max(fives))
}

fn align_coefficient(decimal: Decimal, target_scale: u32) -> Result<i128, ComputeError> {
    let delta = target_scale.checked_sub(decimal.scale).ok_or_else(|| {
        ComputeError::precision_issue("target scale is smaller than decimal scale")
    })?;
    decimal
        .coefficient
        .checked_mul(pow10(delta)?)
        .ok_or_else(|| ComputeError::overflow("decimal scale alignment overflow"))
}

fn pow10(exponent: u32) -> Result<i128, ComputeError> {
    if exponent > MAX_DECIMAL_SCALE {
        return Err(ComputeError::precision_issue(format!(
            "decimal scale {exponent} exceeds maximum {MAX_DECIMAL_SCALE}"
        )));
    }

    let mut value = 1_i128;
    for _ in 0..exponent {
        value = value
            .checked_mul(10)
            .ok_or_else(|| ComputeError::overflow("power-of-ten overflow"))?;
    }
    Ok(value)
}

/// Returns the current compute engine status.
#[must_use]
pub fn engine_status() -> EngineStatus {
    EngineStatus::FoundationOnly
}

/// Returns the crate version compiled into the binary.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::{
        add, binary_arithmetic, compute_binary, divide, engine_status, version,
        ArithmeticOperation, ComputeRequest, Decimal, EngineStatus, ErrorCode, Number,
        NumericValue, PrecisionPolicy, RoundingMode,
    };
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn reports_foundation_status() {
        assert_eq!(engine_status(), EngineStatus::FoundationOnly);
    }

    #[test]
    fn exposes_package_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn serializes_request_with_camel_case_precision() -> serde_json::Result<()> {
        let request = ComputeRequest {
            operation: "arithmetic.add".to_owned(),
            input: json!({"left": {"kind": "integer", "value": "2"}}),
            precision: Some(PrecisionPolicy {
                decimal_places: Some(2),
                rounding: RoundingMode::HalfAwayFromZero,
            }),
            trace: true,
        };

        let serialized = serde_json::to_value(request)?;

        assert_eq!(serialized["precision"]["decimalPlaces"], 2);
        assert_eq!(serialized["precision"]["rounding"], "half-away-from-zero");
        Ok(())
    }

    #[test]
    fn serializes_success_and_failure_response_casing() -> serde_json::Result<()> {
        let success = compute_binary(
            ArithmeticOperation::Add,
            Number::Integer(1),
            Number::Integer(2),
            PrecisionPolicy::default(),
            false,
        );
        let success_json = serde_json::to_value(success)?;

        assert_eq!(success_json["ok"], true);
        assert_eq!(
            success_json["result"]["metadata"]["engineVersion"],
            version()
        );
        assert_eq!(success_json["result"]["metadata"]["numericKind"], "integer");

        let failure = compute_binary(
            ArithmeticOperation::Divide,
            Number::Integer(1),
            Number::Integer(0),
            PrecisionPolicy::default(),
            false,
        );
        let failure_json = serde_json::to_value(failure)?;

        assert_eq!(failure_json["ok"], false);
        assert_eq!(failure_json["error"]["code"], "division-by-zero");
        Ok(())
    }

    #[test]
    fn adds_exact_integers() {
        assert_eq!(
            add(Number::Integer(40), Number::Integer(2)),
            Ok(Number::Integer(42))
        );
    }

    #[test]
    fn adds_decimals_with_aligned_scale() -> Result<(), Box<dyn std::error::Error>> {
        let left = Decimal::from_str("1.20")?;
        let right = Decimal::from_str("3.045")?;

        assert_eq!(
            add(Number::Decimal(left), Number::Decimal(right)).map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "4.245".to_owned(),
                scale: 3,
            })
        );
        Ok(())
    }

    #[test]
    fn divides_exact_terminating_decimal() {
        assert_eq!(
            divide(
                Number::Integer(1),
                Number::Integer(4),
                PrecisionPolicy::default(),
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "0.25".to_owned(),
                scale: 2,
            })
        );
    }

    #[test]
    fn rounds_division_with_half_away_from_zero() {
        assert_eq!(
            divide(
                Number::Integer(2),
                Number::Integer(3),
                PrecisionPolicy {
                    decimal_places: Some(2),
                    rounding: RoundingMode::HalfAwayFromZero,
                },
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "0.67".to_owned(),
                scale: 2,
            })
        );
    }

    #[test]
    fn rounds_negative_division_half_away_from_zero() {
        assert_eq!(
            divide(
                Number::Integer(-5),
                Number::Integer(2),
                PrecisionPolicy {
                    decimal_places: Some(0),
                    rounding: RoundingMode::HalfAwayFromZero,
                },
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "-3".to_owned(),
                scale: 0,
            })
        );
    }

    #[test]
    fn reports_i128_min_division_overflow_without_panicking() {
        assert_eq!(
            divide(
                Number::Integer(i128::MIN),
                Number::Integer(-1),
                PrecisionPolicy::default(),
            )
            .map_err(|error| error.code),
            Err(ErrorCode::Overflow)
        );
    }

    #[test]
    fn checked_private_division_helpers_report_i128_min_overflow() {
        assert_eq!(
            super::divide_and_round(i128::MIN, -1, RoundingMode::Exact).map_err(|error| error.code),
            Err(ErrorCode::Overflow)
        );
        assert_eq!(
            super::divide_exact(i128::MIN, -1).map_err(|error| error.code),
            Err(ErrorCode::Overflow)
        );
    }

    #[test]
    fn divides_equal_high_scale_decimals_exactly() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            divide(
                Number::Decimal(Decimal::with_scale(1, 38)?),
                Number::Decimal(Decimal::with_scale(1, 38)?),
                PrecisionPolicy::default(),
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "1".to_owned(),
                scale: 0,
            })
        );
        Ok(())
    }

    #[test]
    fn divides_equal_high_scale_ten_coefficients_exactly() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_eq!(
            divide(
                Number::Decimal(Decimal::with_scale(10, 38)?),
                Number::Decimal(Decimal::with_scale(10, 38)?),
                PrecisionPolicy::default(),
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "1".to_owned(),
                scale: 0,
            })
        );
        Ok(())
    }

    #[test]
    fn divides_equal_high_scale_max_coefficients_exactly() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_eq!(
            divide(
                Number::Decimal(Decimal::with_scale(i128::MAX, 38)?),
                Number::Decimal(Decimal::with_scale(i128::MAX, 38)?),
                PrecisionPolicy::default(),
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "1".to_owned(),
                scale: 0,
            })
        );
        Ok(())
    }

    #[test]
    fn divides_equal_i128_min_coefficients_exactly() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            divide(
                Number::Decimal(Decimal::with_scale(i128::MIN, 0)?),
                Number::Decimal(Decimal::with_scale(i128::MIN, 0)?),
                PrecisionPolicy::default(),
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "1".to_owned(),
                scale: 0,
            })
        );
        Ok(())
    }

    #[test]
    fn divides_unequal_scale_exact_case_by_cancelling_before_multiplication(
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            divide(
                Number::Decimal(Decimal::with_scale(i128::MAX, 38)?),
                Number::Decimal(Decimal::with_scale(i128::MAX, 37)?),
                PrecisionPolicy::default(),
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "0.1".to_owned(),
                scale: 1,
            })
        );
        Ok(())
    }

    #[test]
    fn preserves_requested_decimal_places() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            binary_arithmetic(
                ArithmeticOperation::Add,
                Number::Decimal(Decimal::with_scale(120, 2)?),
                Number::Decimal(Decimal::with_scale(0, 2)?),
                PrecisionPolicy {
                    decimal_places: Some(2),
                    rounding: RoundingMode::Exact,
                },
            )
            .map(NumericValue::from),
            Ok(NumericValue::Decimal {
                value: "1.20".to_owned(),
                scale: 2,
            })
        );
        Ok(())
    }

    #[test]
    fn rejects_repeating_decimal_without_rounding_policy() {
        assert_eq!(
            divide(
                Number::Integer(1),
                Number::Integer(3),
                PrecisionPolicy::default(),
            )
            .map_err(|error| error.code),
            Err(ErrorCode::PrecisionIssue)
        );
    }

    #[test]
    fn rejects_precision_loss_when_rounding_is_exact() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            binary_arithmetic(
                ArithmeticOperation::Multiply,
                Number::Decimal(Decimal::from_str("1.234")?),
                Number::Integer(1),
                PrecisionPolicy {
                    decimal_places: Some(2),
                    rounding: RoundingMode::Exact,
                },
            )
            .map_err(|error| error.code),
            Err(ErrorCode::PrecisionIssue)
        );
        Ok(())
    }

    #[test]
    fn rejects_division_by_zero() {
        assert_eq!(
            divide(
                Number::Integer(1),
                Number::Integer(0),
                PrecisionPolicy::default(),
            )
            .map_err(|error| error.code),
            Err(ErrorCode::DivisionByZero)
        );
    }

    #[test]
    fn reports_integer_overflow_paths() {
        assert_eq!(
            add(Number::Integer(i128::MAX), Number::Integer(1)).map_err(|error| error.code),
            Err(ErrorCode::Overflow)
        );
        assert_eq!(
            super::multiply(Number::Integer(i128::MAX), Number::Integer(2))
                .map_err(|error| error.code),
            Err(ErrorCode::Overflow)
        );
    }

    #[test]
    fn reports_decimal_overflow_paths() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            super::multiply(
                Number::Decimal(Decimal::with_scale(i128::MAX, 0)?),
                Number::Decimal(Decimal::with_scale(2, 0)?),
            )
            .map_err(|error| error.code),
            Err(ErrorCode::Overflow)
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_decimal_input() {
        assert_eq!(
            Decimal::from_str("12.3.4").map_err(|error| error.code),
            Err(ErrorCode::InvalidInput)
        );
    }

    #[test]
    fn rejects_numeric_value_scale_mismatch() {
        assert_eq!(
            NumericValue::Decimal {
                value: "1.2".to_owned(),
                scale: 5,
            }
            .parse_number()
            .map_err(|error| error.code),
            Err(ErrorCode::InvalidInput)
        );
    }

    #[test]
    fn parses_i128_min_decimal_numeric_value() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            NumericValue::Decimal {
                value: i128::MIN.to_string(),
                scale: 0,
            }
            .parse_number(),
            Ok(Number::Decimal(Decimal::with_scale(i128::MIN, 0)?))
        );
        Ok(())
    }

    #[test]
    fn serde_round_trips_i128_min_decimal_numeric_value() -> serde_json::Result<()> {
        let value = NumericValue::Decimal {
            value: i128::MIN.to_string(),
            scale: 0,
        };

        let serialized = serde_json::to_value(&value)?;
        let deserialized = serde_json::from_value::<NumericValue>(serialized)?;

        assert_eq!(deserialized, value);
        Ok(())
    }

    #[test]
    fn rejects_oversized_positive_decimal_numeric_value() {
        let value = "170141183460469231731687303715884105728";

        assert_eq!(
            NumericValue::Decimal {
                value: value.to_owned(),
                scale: 0,
            }
            .parse_number()
            .map_err(|error| error.code),
            Err(ErrorCode::InvalidInput)
        );

        let result = serde_json::from_value::<NumericValue>(json!({
            "kind": "decimal",
            "value": value,
            "scale": 0
        }));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_deserialized_numeric_value_scale_mismatch() {
        let result = serde_json::from_value::<NumericValue>(json!({
            "kind": "decimal",
            "value": "1.2",
            "scale": 5
        }));

        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_public_decimal_scale_construction() {
        assert_eq!(
            Decimal::with_scale(1, 39).map_err(|error| error.code),
            Err(ErrorCode::PrecisionIssue)
        );
        assert_eq!(
            Decimal::new(1, 39).map_err(|error| error.code),
            Err(ErrorCode::PrecisionIssue)
        );
    }

    #[test]
    fn emits_deterministic_response_metadata_and_trace() {
        let response = compute_binary(
            ArithmeticOperation::Add,
            Number::Integer(2),
            Number::Integer(2),
            PrecisionPolicy::default(),
            true,
        );

        assert!(response.ok);
        assert_eq!(
            response.result.map(|result| (
                result.operation,
                result.metadata.deterministic,
                result.metadata.engine_version,
                result.value
            )),
            Some((
                "arithmetic.add".to_owned(),
                true,
                version().to_owned(),
                NumericValue::Integer {
                    value: "4".to_owned(),
                }
            ))
        );
        assert_eq!(response.trace.map(|trace| trace.len()), Some(1));
    }
}
