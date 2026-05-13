//! Deterministic unit conversion and dimensional analysis.

use crate::{
    version, ComputeError, Decimal, ErrorCode, Number, NumericValue, PrecisionPolicy, RoundingMode,
};
use serde::{Deserialize, Serialize};

const UNITS_OPERATION_ID: &str = "units.convert";
const MAX_DECIMAL_SCALE: u32 = 38;
const MAX_UNKNOWN_UNIT_DETAIL_CHARS: usize = 64;

/// Supported physical dimensions for unit conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitDimension {
    Length,
    Mass,
    Time,
    Temperature,
}

impl UnitDimension {
    fn as_str(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Mass => "mass",
            Self::Time => "time",
            Self::Temperature => "temperature",
        }
    }
}

/// Deterministic conversion result suitable for CLI/MCP wrapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitConversionResult {
    pub operation: String,
    pub value: NumericValue,
    pub dimension: UnitDimension,
    pub metadata: UnitConversionMetadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<UnitTraceStep>,
}

/// Stable unit conversion metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitConversionMetadata {
    pub operation: String,
    pub engine_version: String,
    pub deterministic: bool,
    pub source_unit: String,
    pub target_unit: String,
    pub conversion_kind: UnitConversionKind,
    pub factor_numerator: String,
    pub factor_denominator: String,
    pub scale_numerator: String,
    pub scale_denominator: String,
    pub offset_numerator: String,
    pub offset_denominator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
    pub precision: PrecisionPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<String>,
}

/// Unit conversion equation shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitConversionKind {
    Linear,
    Affine,
}

/// Deterministic unit conversion trace step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitTraceStep {
    pub step: u32,
    pub operation: String,
    pub input: NumericValue,
    pub output: NumericValue,
    pub note: String,
}

/// Converts a deterministic numeric value between compatible units.
pub fn convert_units(
    value: Number,
    source_unit: &str,
    target_unit: &str,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<UnitConversionResult, ComputeError> {
    let source = parse_unit(source_unit)?;
    let target = parse_unit(target_unit)?;

    if source.dimension != target.dimension {
        return Err(incompatible_dimensions(source, target));
    }

    let mut trace = UnitTrace::new(include_trace);
    trace.record(
        "units.parse",
        value,
        value,
        format!(
            "parsed {} and {} as {} units",
            source.symbol,
            target.symbol,
            source.dimension.as_str()
        ),
    );

    let transform = if source.dimension == UnitDimension::Temperature {
        temperature_transform(source, target)?
    } else {
        let (numerator, denominator) = conversion_factor(source, target)?;
        UnitTransform::linear(numerator, denominator)?
    };

    let converted = apply_transform(value, transform, precision)?;
    trace.record(
        transform.trace_operation(),
        value,
        converted,
        transform.trace_note(source, target),
    );

    Ok(UnitConversionResult {
        operation: UNITS_OPERATION_ID.to_owned(),
        value: converted.into(),
        dimension: source.dimension,
        metadata: UnitConversionMetadata {
            operation: UNITS_OPERATION_ID.to_owned(),
            engine_version: version().to_owned(),
            deterministic: true,
            source_unit: source.symbol.to_owned(),
            target_unit: target.symbol.to_owned(),
            conversion_kind: transform.kind,
            factor_numerator: transform.scale.numerator.to_string(),
            factor_denominator: transform.scale.denominator.to_string(),
            scale_numerator: transform.scale.numerator.to_string(),
            scale_denominator: transform.scale.denominator.to_string(),
            offset_numerator: transform.offset.numerator.to_string(),
            offset_denominator: transform.offset.denominator.to_string(),
            offset: transform.offset_note(),
            precision,
            assumptions: assumptions(source, target),
        },
        trace: trace.steps,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnitDefinition {
    symbol: &'static str,
    dimension: UnitDimension,
    numerator: i128,
    denominator: i128,
}

fn parse_unit(input: &str) -> Result<UnitDefinition, ComputeError> {
    match input.trim() {
        "m" | "meter" | "meters" => Ok(linear("m", UnitDimension::Length, 1, 1)),
        "cm" | "centimeter" | "centimeters" => Ok(linear("cm", UnitDimension::Length, 1, 100)),
        "km" | "kilometer" | "kilometers" => Ok(linear("km", UnitDimension::Length, 1000, 1)),
        "in" | "inch" | "inches" => Ok(linear("in", UnitDimension::Length, 127, 5000)),
        "ft" | "foot" | "feet" => Ok(linear("ft", UnitDimension::Length, 381, 1250)),
        "g" | "gram" | "grams" => Ok(linear("g", UnitDimension::Mass, 1, 1000)),
        "kg" | "kilogram" | "kilograms" => Ok(linear("kg", UnitDimension::Mass, 1, 1)),
        "lb" | "pound" | "pounds" => Ok(linear("lb", UnitDimension::Mass, 45359237, 100000000)),
        "s" | "sec" | "second" | "seconds" => Ok(linear("s", UnitDimension::Time, 1, 1)),
        "min" | "minute" | "minutes" => Ok(linear("min", UnitDimension::Time, 60, 1)),
        "h" | "hr" | "hour" | "hours" => Ok(linear("h", UnitDimension::Time, 3600, 1)),
        "C" | "degC" | "celsius" => Ok(temperature("C")),
        "F" | "degF" | "fahrenheit" => Ok(temperature("F")),
        "K" | "kelvin" => Ok(temperature("K")),
        _ => Err(unknown_unit(input)),
    }
}

fn linear(
    symbol: &'static str,
    dimension: UnitDimension,
    numerator: i128,
    denominator: i128,
) -> UnitDefinition {
    UnitDefinition {
        symbol,
        dimension,
        numerator,
        denominator,
    }
}

fn temperature(symbol: &'static str) -> UnitDefinition {
    UnitDefinition {
        symbol,
        dimension: UnitDimension::Temperature,
        numerator: 1,
        denominator: 1,
    }
}

fn conversion_factor(
    source: UnitDefinition,
    target: UnitDefinition,
) -> Result<(i128, i128), ComputeError> {
    let numerator = source
        .numerator
        .checked_mul(target.denominator)
        .ok_or_else(|| ComputeError::overflow("unit conversion factor overflow"))?;
    let denominator = source
        .denominator
        .checked_mul(target.numerator)
        .ok_or_else(|| ComputeError::overflow("unit conversion factor overflow"))?;
    reduce_positive_fraction(numerator, denominator)
}

fn reduce_positive_fraction(
    numerator: i128,
    denominator: i128,
) -> Result<(i128, i128), ComputeError> {
    let divisor = gcd_i128_divisor(numerator.unsigned_abs(), denominator.unsigned_abs())?;
    Ok((numerator / divisor, denominator / divisor))
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn gcd_i128_divisor(left: u128, right: u128) -> Result<i128, ComputeError> {
    let divisor = gcd(left, right);
    i128::try_from(divisor)
        .map_err(|_| ComputeError::overflow("unit conversion reduction divisor overflow"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    fn new(numerator: i128, denominator: i128) -> Result<Self, ComputeError> {
        if denominator == 0 {
            return Err(ComputeError::division_by_zero());
        }

        let normalized = if denominator < 0 {
            Self {
                numerator: numerator
                    .checked_neg()
                    .ok_or_else(|| ComputeError::overflow("rational numerator sign overflow"))?,
                denominator: denominator
                    .checked_neg()
                    .ok_or_else(|| ComputeError::overflow("rational denominator sign overflow"))?,
            }
        } else {
            Self {
                numerator,
                denominator,
            }
        };
        normalized.reduced()
    }

    fn integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    fn zero() -> Self {
        Self::integer(0)
    }

    fn reduced(self) -> Result<Self, ComputeError> {
        if self.numerator == 0 {
            return Ok(Self::zero());
        }

        let divisor = gcd_i128_divisor(
            self.numerator.unsigned_abs(),
            self.denominator.unsigned_abs(),
        )?;
        Ok(Self {
            numerator: self.numerator / divisor,
            denominator: self.denominator / divisor,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnitTransform {
    kind: UnitConversionKind,
    scale: Rational,
    offset: Rational,
}

impl UnitTransform {
    fn linear(numerator: i128, denominator: i128) -> Result<Self, ComputeError> {
        Ok(Self {
            kind: UnitConversionKind::Linear,
            scale: Rational::new(numerator, denominator)?,
            offset: Rational::zero(),
        })
    }

    fn affine(scale: Rational, offset: Rational) -> Self {
        Self {
            kind: UnitConversionKind::Affine,
            scale,
            offset,
        }
    }

    fn trace_operation(self) -> &'static str {
        match self.kind {
            UnitConversionKind::Linear => "units.apply-factor",
            UnitConversionKind::Affine => "units.apply-affine",
        }
    }

    fn trace_note(self, source: UnitDefinition, target: UnitDefinition) -> String {
        match self.kind {
            UnitConversionKind::Linear => format!(
                "apply reduced factor {}/{} from {} to {}",
                self.scale.numerator, self.scale.denominator, source.symbol, target.symbol
            ),
            UnitConversionKind::Affine => format!(
                "apply affine transform value * {}/{} + {}/{} from {} to {}",
                self.scale.numerator,
                self.scale.denominator,
                self.offset.numerator,
                self.offset.denominator,
                source.symbol,
                target.symbol
            ),
        }
    }

    fn offset_note(self) -> Option<String> {
        (self.kind == UnitConversionKind::Affine).then(|| {
            format!(
                "{}/{} in target unit after scale",
                self.offset.numerator, self.offset.denominator
            )
        })
    }
}

fn temperature_transform(
    source: UnitDefinition,
    target: UnitDefinition,
) -> Result<UnitTransform, ComputeError> {
    if source.symbol == target.symbol {
        return UnitTransform::linear(1, 1);
    }

    let (scale_numerator, scale_denominator, offset_numerator, offset_denominator) =
        match (source.symbol, target.symbol) {
            ("C", "F") => (9, 5, 32, 1),
            ("F", "C") => (5, 9, -160, 9),
            ("C", "K") => (1, 1, 27315, 100),
            ("K", "C") => (1, 1, -27315, 100),
            ("F", "K") => (5, 9, 45967, 180),
            ("K", "F") => (9, 5, -45967, 100),
            _ => return Err(unknown_unit(source.symbol)),
        };

    Ok(UnitTransform::affine(
        Rational::new(scale_numerator, scale_denominator)?,
        Rational::new(offset_numerator, offset_denominator)?,
    ))
}

fn apply_transform(
    value: Number,
    transform: UnitTransform,
    precision: PrecisionPolicy,
) -> Result<Number, ComputeError> {
    let value = number_rational(value)?;
    let scaled = multiply_rationals(value, transform.scale)?;
    let shifted = add_rationals(scaled, transform.offset)?;
    rational_to_number(shifted, precision)
}

fn number_rational(value: Number) -> Result<Rational, ComputeError> {
    match value {
        Number::Integer(value) => Ok(Rational::integer(value)),
        Number::Decimal(value) => Rational::new(value.coefficient(), pow10(value.scale())?),
    }
}

fn multiply_rationals(left: Rational, right: Rational) -> Result<Rational, ComputeError> {
    let mut left_numerator = left.numerator;
    let mut right_numerator = right.numerator;
    let mut left_denominator = left.denominator;
    let mut right_denominator = right.denominator;

    reduce_pair(&mut left_numerator, &mut right_denominator)?;
    reduce_pair(&mut right_numerator, &mut left_denominator)?;

    let numerator = left_numerator
        .checked_mul(right_numerator)
        .ok_or_else(|| ComputeError::overflow("unit conversion numerator overflow"))?;
    let denominator = left_denominator
        .checked_mul(right_denominator)
        .ok_or_else(|| ComputeError::overflow("unit conversion denominator overflow"))?;
    Rational::new(numerator, denominator)
}

fn add_rationals(left: Rational, right: Rational) -> Result<Rational, ComputeError> {
    let divisor = gcd_i128_divisor(
        left.denominator.unsigned_abs(),
        right.denominator.unsigned_abs(),
    )?;
    let left_multiplier = checked_div_i128(right.denominator, divisor)?;
    let right_multiplier = checked_div_i128(left.denominator, divisor)?;

    let left_numerator = left
        .numerator
        .checked_mul(left_multiplier)
        .ok_or_else(|| ComputeError::overflow("unit conversion affine numerator overflow"))?;
    let right_numerator = right
        .numerator
        .checked_mul(right_multiplier)
        .ok_or_else(|| ComputeError::overflow("unit conversion affine numerator overflow"))?;
    let numerator = left_numerator
        .checked_add(right_numerator)
        .ok_or_else(|| ComputeError::overflow("unit conversion affine numerator overflow"))?;
    let denominator = left
        .denominator
        .checked_mul(left_multiplier)
        .ok_or_else(|| ComputeError::overflow("unit conversion affine denominator overflow"))?;

    Rational::new(numerator, denominator)
}

fn reduce_pair(numerator: &mut i128, denominator: &mut i128) -> Result<(), ComputeError> {
    let divisor = gcd_i128_divisor(numerator.unsigned_abs(), denominator.unsigned_abs())?;
    if divisor > 1 {
        *numerator /= divisor;
        *denominator /= divisor;
    }
    Ok(())
}

fn rational_to_number(
    rational: Rational,
    precision: PrecisionPolicy,
) -> Result<Number, ComputeError> {
    if let Some(decimal_places) = precision.decimal_places {
        let quotient = scaled_divide_and_round(
            rational.numerator,
            rational.denominator,
            pow10(decimal_places)?,
            precision.rounding,
        )?;
        return Decimal::with_scale(quotient, decimal_places).map(Number::Decimal);
    }

    let extra_scale = terminating_scale(rational.denominator)?;
    let quotient = scaled_divide_and_round(
        rational.numerator,
        rational.denominator,
        pow10(extra_scale)?,
        RoundingMode::Exact,
    )?;
    Decimal::new(quotient, extra_scale).map(Number::Decimal)
}

fn scaled_divide_and_round(
    numerator: i128,
    denominator: i128,
    scale: i128,
    rounding: RoundingMode,
) -> Result<i128, ComputeError> {
    let mut reduced_scale = scale;
    let mut reduced_denominator = denominator;
    reduce_pair(&mut reduced_scale, &mut reduced_denominator)?;

    if let Some(scaled_numerator) = numerator.checked_mul(reduced_scale) {
        return divide_and_round(scaled_numerator, reduced_denominator, rounding);
    }

    let quotient = checked_div_i128(numerator, reduced_denominator)?;
    let remainder = checked_rem_i128(numerator, reduced_denominator)?;
    let scaled_quotient = quotient
        .checked_mul(reduced_scale)
        .ok_or_else(|| ComputeError::overflow("unit conversion precision scaling overflow"))?;
    let scaled_remainder =
        scaled_remainder_quotient(remainder, reduced_denominator, reduced_scale, rounding)?;
    scaled_quotient
        .checked_add(scaled_remainder)
        .ok_or_else(|| ComputeError::overflow("unit conversion precision scaling overflow"))
}

fn scaled_remainder_quotient(
    remainder: i128,
    denominator: i128,
    scale: i128,
    rounding: RoundingMode,
) -> Result<i128, ComputeError> {
    if remainder == 0 {
        return Ok(0);
    }

    let mut reduced_remainder = remainder;
    let mut reduced_denominator = denominator;
    reduce_pair(&mut reduced_remainder, &mut reduced_denominator)?;

    let mut reduced_scale = scale;
    reduce_pair(&mut reduced_scale, &mut reduced_denominator)?;

    let scaled_remainder = reduced_remainder
        .checked_mul(reduced_scale)
        .ok_or_else(|| ComputeError::overflow("unit conversion precision scaling overflow"))?;
    divide_and_round(scaled_remainder, reduced_denominator, rounding)
}

fn divide_and_round(
    numerator: i128,
    denominator: i128,
    rounding: RoundingMode,
) -> Result<i128, ComputeError> {
    let quotient = checked_div_i128(numerator, denominator)?;
    let remainder = checked_rem_i128(numerator, denominator)?;
    if remainder == 0 {
        return Ok(quotient);
    }

    match rounding {
        RoundingMode::Exact => Err(ComputeError::precision_issue(
            "result cannot be represented exactly with requested precision",
        )),
        RoundingMode::Truncate => Ok(quotient),
        RoundingMode::HalfAwayFromZero => {
            let should_increment = remainder
                .unsigned_abs()
                .checked_mul(2)
                .is_some_and(|doubled| doubled >= denominator.unsigned_abs());
            if should_increment {
                let sign = if (numerator < 0) == (denominator < 0) {
                    1
                } else {
                    -1
                };
                quotient
                    .checked_add(sign)
                    .ok_or_else(|| ComputeError::overflow("rounded unit conversion overflow"))
            } else {
                Ok(quotient)
            }
        }
    }
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
        return Err(ComputeError::precision_issue(
            "division result has a repeating decimal expansion",
        ));
    }

    Ok(twos.max(fives))
}

fn checked_div_i128(numerator: i128, denominator: i128) -> Result<i128, ComputeError> {
    numerator
        .checked_div(denominator)
        .ok_or_else(|| ComputeError::overflow("unit conversion division overflow"))
}

fn checked_rem_i128(numerator: i128, denominator: i128) -> Result<i128, ComputeError> {
    numerator
        .checked_rem(denominator)
        .ok_or_else(|| ComputeError::overflow("unit conversion remainder overflow"))
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
            .ok_or_else(|| ComputeError::overflow("unit conversion power-of-ten overflow"))?;
    }
    Ok(value)
}

fn assumptions(source: UnitDefinition, target: UnitDefinition) -> Vec<String> {
    [source.symbol, target.symbol]
        .into_iter()
        .filter_map(unit_assumption)
        .fold(Vec::new(), |mut assumptions, assumption| {
            if !assumptions.iter().any(|existing| existing == assumption) {
                assumptions.push(assumption.to_owned());
            }
            assumptions
        })
}

fn unit_assumption(symbol: &str) -> Option<&'static str> {
    match symbol {
        "in" => Some("1 in = 0.0254 m"),
        "ft" => Some("1 ft = 0.3048 m"),
        "lb" => Some("1 lb = 0.45359237 kg"),
        _ => None,
    }
}

fn incompatible_dimensions(source: UnitDefinition, target: UnitDefinition) -> ComputeError {
    ComputeError {
        code: ErrorCode::InvalidInput,
        message: "incompatible unit dimensions".to_owned(),
        detail: Some(format!(
            "source dimension {} is incompatible with target dimension {}",
            source.dimension.as_str(),
            target.dimension.as_str()
        )),
    }
}

fn unknown_unit(unit: &str) -> ComputeError {
    let trimmed = unit.trim();
    let mut detail = trimmed
        .chars()
        .take(MAX_UNKNOWN_UNIT_DETAIL_CHARS)
        .collect::<String>();
    if trimmed.chars().count() > MAX_UNKNOWN_UNIT_DETAIL_CHARS {
        detail.push_str("...");
    }

    ComputeError {
        code: ErrorCode::InvalidInput,
        message: "unknown unit".to_owned(),
        detail: Some(format!("unknown unit: {detail}")),
    }
}

struct UnitTrace {
    include: bool,
    next_step: u32,
    steps: Vec<UnitTraceStep>,
}

impl UnitTrace {
    fn new(include: bool) -> Self {
        Self {
            include,
            next_step: 1,
            steps: Vec::new(),
        }
    }

    fn record(
        &mut self,
        operation: impl Into<String>,
        input: Number,
        output: Number,
        note: impl Into<String>,
    ) {
        if !self.include {
            return;
        }

        self.steps.push(UnitTraceStep {
            step: self.next_step,
            operation: operation.into(),
            input: input.into(),
            output: output.into(),
            note: note.into(),
        });
        self.next_step += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decimal, ErrorCode, Number, NumericValue, PrecisionPolicy, RoundingMode};
    use serde_json::json;
    use std::str::FromStr;

    fn exact_scale(decimal_places: u32) -> PrecisionPolicy {
        PrecisionPolicy {
            decimal_places: Some(decimal_places),
            rounding: RoundingMode::Exact,
        }
    }

    fn round_scale(decimal_places: u32) -> PrecisionPolicy {
        PrecisionPolicy {
            decimal_places: Some(decimal_places),
            rounding: RoundingMode::HalfAwayFromZero,
        }
    }

    #[test]
    fn converts_exact_metric_length() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(
            Number::Integer(1500),
            "m",
            "km",
            PrecisionPolicy::default(),
            true,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "1.5".to_owned(),
                scale: 1,
            }
        );
        assert_eq!(result.dimension, UnitDimension::Length);
        assert_eq!(result.metadata.factor_numerator, "1".to_owned());
        assert_eq!(result.metadata.factor_denominator, "1000".to_owned());
        assert_eq!(result.metadata.conversion_kind, UnitConversionKind::Linear);
        assert_eq!(result.trace.len(), 2);
        assert_eq!(result.trace[0].operation, "units.parse");
        assert_eq!(result.trace[1].operation, "units.apply-factor");
        Ok(())
    }

    #[test]
    fn converts_exact_imperial_to_metric_length() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(
            Number::Integer(12),
            "in",
            "ft",
            PrecisionPolicy::default(),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "1".to_owned(),
                scale: 0,
            }
        );
        assert!(result.trace.is_empty());
        Ok(())
    }

    #[test]
    fn applies_reduced_linear_factor_before_multiplication(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let feet = i128::MAX / 100;
        let result = convert_units(
            Number::Integer(feet),
            "ft",
            "in",
            PrecisionPolicy::default(),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: (feet * 12).to_string(),
                scale: 0,
            }
        );
        assert_eq!(result.metadata.factor_numerator, "12");
        assert_eq!(result.metadata.factor_denominator, "1");
        Ok(())
    }

    #[test]
    fn applies_reduced_factor_with_requested_precision() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(
            Number::Decimal(Decimal::from_str("2.5")?),
            "ft",
            "in",
            exact_scale(1),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "30.0".to_owned(),
                scale: 1,
            }
        );
        assert_eq!(result.metadata.scale_numerator, "12");
        assert_eq!(result.metadata.scale_denominator, "1");
        Ok(())
    }

    #[test]
    fn scales_precision_after_reducing_denominator() -> Result<(), Box<dyn std::error::Error>> {
        let inches = (i128::MAX / 12) * 12;
        let result = convert_units(Number::Integer(inches), "in", "ft", exact_scale(1), false)?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: format!("{}.0", (inches / 6) * 5 / 10),
                scale: 1,
            }
        );
        assert_eq!(result.metadata.factor_numerator, "1");
        assert_eq!(result.metadata.factor_denominator, "12");
        Ok(())
    }

    #[test]
    fn handles_i128_min_input_with_reduction_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(Number::Integer(i128::MIN), "cm", "m", exact_scale(2), false)?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "-1701411834604692317316873037158841057.28".to_owned(),
                scale: 2,
            }
        );
        Ok(())
    }

    #[test]
    fn default_precision_scales_i128_min_without_overflow() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = convert_units(
            Number::Integer(i128::MIN),
            "cm",
            "m",
            PrecisionPolicy::default(),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "-1701411834604692317316873037158841057.28".to_owned(),
                scale: 2,
            }
        );
        Ok(())
    }

    #[test]
    fn converts_mass_with_decimal_factor() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(Number::Integer(1), "lb", "kg", exact_scale(8), true)?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "0.45359237".to_owned(),
                scale: 8,
            }
        );
        assert_eq!(result.dimension, UnitDimension::Mass);
        assert_eq!(
            result.metadata.assumptions,
            vec!["1 lb = 0.45359237 kg".to_owned()]
        );
        Ok(())
    }

    #[test]
    fn converts_time_exactly() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(
            Number::Integer(2),
            "h",
            "min",
            PrecisionPolicy::default(),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "120".to_owned(),
                scale: 0,
            }
        );
        Ok(())
    }

    #[test]
    fn applies_rounding_policy_for_repeating_conversion() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = convert_units(Number::Integer(1), "m", "ft", round_scale(3), true)?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "3.281".to_owned(),
                scale: 3,
            }
        );
        assert_eq!(result.metadata.precision, round_scale(3));
        Ok(())
    }

    #[test]
    fn rejects_precision_loss_without_rounding_policy() {
        let error = convert_units(
            Number::Integer(1),
            "m",
            "ft",
            PrecisionPolicy::default(),
            false,
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.code),
            Some(ErrorCode::PrecisionIssue)
        );
    }

    #[test]
    fn rejects_incompatible_dimensions() {
        let error = convert_units(
            Number::Integer(1),
            "m",
            "kg",
            PrecisionPolicy::default(),
            false,
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
        assert_eq!(
            error.and_then(|error| error.detail),
            Some("source dimension length is incompatible with target dimension mass".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_units() {
        let error = convert_units(
            Number::Integer(1),
            "parsec",
            "m",
            PrecisionPolicy::default(),
            false,
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
        assert_eq!(
            error.and_then(|error| error.detail),
            Some("unknown unit: parsec".to_owned())
        );
    }

    #[test]
    fn truncates_unknown_unit_error_detail() {
        let long_unit = "x".repeat(80);
        let error = convert_units(
            Number::Integer(1),
            &long_unit,
            "m",
            PrecisionPolicy::default(),
            false,
        )
        .err();

        assert_eq!(
            error.and_then(|error| error.detail),
            Some(format!("unknown unit: {}...", "x".repeat(64)))
        );
    }

    #[test]
    fn converts_temperature_offsets_exactly() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(Number::Integer(32), "F", "C", exact_scale(2), true)?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "0.00".to_owned(),
                scale: 2,
            }
        );
        assert_eq!(result.dimension, UnitDimension::Temperature);
        assert_eq!(result.metadata.conversion_kind, UnitConversionKind::Affine);
        assert_eq!(result.metadata.scale_numerator, "5");
        assert_eq!(result.metadata.scale_denominator, "9");
        assert_eq!(result.metadata.offset_numerator, "-160");
        assert_eq!(result.metadata.offset_denominator, "9");
        assert_eq!(
            result.metadata.offset,
            Some("-160/9 in target unit after scale".to_owned())
        );
        assert_eq!(result.trace[1].operation, "units.apply-affine");
        Ok(())
    }

    #[test]
    fn converts_temperature_with_rounding() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(
            Number::Decimal(Decimal::from_str("98.6")?),
            "F",
            "C",
            round_scale(1),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "37.0".to_owned(),
                scale: 1,
            }
        );
        Ok(())
    }

    #[test]
    fn uses_linear_noop_metadata_for_temperature_identity() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = convert_units(Number::Integer(12), "C", "C", exact_scale(1), true)?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "12.0".to_owned(),
                scale: 1,
            }
        );
        assert_eq!(result.metadata.conversion_kind, UnitConversionKind::Linear);
        assert_eq!(result.metadata.scale_numerator, "1");
        assert_eq!(result.metadata.scale_denominator, "1");
        assert_eq!(result.metadata.offset_numerator, "0");
        assert_eq!(result.metadata.offset_denominator, "1");
        assert_eq!(result.metadata.offset, None);
        assert_eq!(result.trace[1].operation, "units.apply-factor");
        Ok(())
    }

    #[test]
    fn serializes_temperature_identity_metadata_and_trace() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = convert_units(Number::Integer(12), "C", "C", exact_scale(1), true)?;
        let serialized = serde_json::to_value(result)?;

        assert_eq!(serialized["metadata"]["conversionKind"], "linear");
        assert_eq!(serialized["metadata"]["offsetNumerator"], "0");
        assert_eq!(serialized["metadata"]["offsetDenominator"], "1");
        assert!(serialized["metadata"].get("offset").is_none());
        assert_eq!(serialized["trace"][1]["operation"], "units.apply-factor");
        Ok(())
    }

    #[test]
    fn converts_fahrenheit_to_kelvin_with_single_final_rounding(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(Number::Integer(33), "F", "K", round_scale(1), false)?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "273.7".to_owned(),
                scale: 1,
            }
        );
        assert_eq!(result.metadata.conversion_kind, UnitConversionKind::Affine);
        assert_eq!(result.metadata.scale_numerator, "5");
        assert_eq!(result.metadata.scale_denominator, "9");
        assert_eq!(result.metadata.offset_numerator, "45967");
        assert_eq!(result.metadata.offset_denominator, "180");
        Ok(())
    }

    #[test]
    fn converts_kelvin_to_fahrenheit_with_single_final_rounding(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(
            Number::Decimal(Decimal::from_str("273.43")?),
            "K",
            "F",
            round_scale(0),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "33".to_owned(),
                scale: 0,
            }
        );
        assert_eq!(result.metadata.conversion_kind, UnitConversionKind::Affine);
        assert_eq!(result.metadata.scale_numerator, "9");
        assert_eq!(result.metadata.scale_denominator, "5");
        assert_eq!(result.metadata.offset_numerator, "-45967");
        assert_eq!(result.metadata.offset_denominator, "100");
        Ok(())
    }

    #[test]
    fn serializes_affine_metadata_and_trace_fields() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(Number::Integer(32), "F", "K", exact_scale(2), true)?;
        let serialized = serde_json::to_value(result)?;

        assert_eq!(
            serialized["metadata"],
            json!({
                "operation": "units.convert",
                "engineVersion": version(),
                "deterministic": true,
                "sourceUnit": "F",
                "targetUnit": "K",
                "conversionKind": "affine",
                "factorNumerator": "5",
                "factorDenominator": "9",
                "scaleNumerator": "5",
                "scaleDenominator": "9",
                "offsetNumerator": "45967",
                "offsetDenominator": "180",
                "offset": "45967/180 in target unit after scale",
                "precision": {
                    "decimalPlaces": 2,
                    "rounding": "exact"
                }
            })
        );
        assert_eq!(serialized["trace"][0]["operation"], "units.parse");
        assert_eq!(serialized["trace"][1]["operation"], "units.apply-affine");
        Ok(())
    }

    #[test]
    fn rejects_lowercase_temperature_symbols() {
        for unit in ["c", "f", "k"] {
            let error = convert_units(
                Number::Integer(1),
                unit,
                "K",
                PrecisionPolicy::default(),
                false,
            )
            .err();

            assert_eq!(
                error.as_ref().map(|error| error.code),
                Some(ErrorCode::InvalidInput)
            );
            assert_eq!(
                error.and_then(|error| error.detail),
                Some(format!("unknown unit: {unit}"))
            );
        }
    }

    #[test]
    fn accepts_lowercase_temperature_names() -> Result<(), Box<dyn std::error::Error>> {
        let result = convert_units(
            Number::Integer(0),
            "celsius",
            "kelvin",
            exact_scale(2),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "273.15".to_owned(),
                scale: 2,
            }
        );
        Ok(())
    }

    #[test]
    fn reports_unit_conversion_overflow() {
        let error = convert_units(
            Number::Integer(i128::MAX),
            "ft",
            "in",
            PrecisionPolicy::default(),
            false,
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.code),
            Some(ErrorCode::Overflow)
        );
    }

    #[test]
    fn emits_deterministic_metadata_and_trace() -> Result<(), Box<dyn std::error::Error>> {
        let first = convert_units(Number::Integer(100), "cm", "m", exact_scale(2), true)?;
        let second = convert_units(Number::Integer(100), "cm", "m", exact_scale(2), true)?;

        assert_eq!(first, second);
        assert_eq!(first.metadata.operation, "units.convert");
        assert!(first.metadata.deterministic);
        assert_eq!(first.metadata.source_unit, "cm");
        assert_eq!(first.metadata.target_unit, "m");
        Ok(())
    }
}
