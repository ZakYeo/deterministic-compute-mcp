//! Deterministic result verification for JSON-safe numeric values.

use crate::{
    align_coefficient, multiply, subtract, version, ComputeError, Decimal, Number, NumericValue,
    TraceStep,
};
use serde::{Deserialize, Serialize};

const VERIFICATION_OPERATION_ID: &str = "verification.compare";

/// Verification tolerance mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Tolerance {
    /// Difference must be less than or equal to this absolute value.
    Absolute { value: NumericValue },
    /// Difference must be less than or equal to `abs(expected) * value`.
    Relative { value: NumericValue },
}

/// Deterministic verification result suitable for generic response wrapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub operation: String,
    pub difference: NumericValue,
    pub metadata: VerificationMetadata,
    pub details: VerificationDetails,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<TraceStep>,
}

/// Stable metadata emitted by verification comparisons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMetadata {
    pub engine_version: String,
    pub deterministic: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<String>,
}

/// Structured comparison details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationDetails {
    pub status: ComparisonStatus,
    pub passed: bool,
    pub mode: ComparisonMode,
    pub expected: NumericValue,
    pub actual: NumericValue,
    pub difference: NumericValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<ToleranceDetails>,
}

/// Machine-readable comparison status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparisonStatus {
    ExactMatch,
    ExactMismatch,
    WithinTolerance,
    OutsideTolerance,
}

/// Machine-readable comparison mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparisonMode {
    Exact,
    AbsoluteTolerance,
    RelativeTolerance,
}

/// Tolerance metadata for tolerance-based comparisons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToleranceDetails {
    pub kind: ToleranceKind,
    pub value: NumericValue,
    pub allowed_difference: NumericValue,
}

/// Tolerance kind emitted in result details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToleranceKind {
    Absolute,
    Relative,
}

/// Compares expected and actual numeric values exactly or with a tolerance.
pub fn compare(
    expected: Number,
    actual: Number,
    tolerance: Option<Tolerance>,
    include_trace: bool,
) -> Result<VerificationResult, ComputeError> {
    let expected_value = NumericValue::from(expected);
    let actual_value = NumericValue::from(actual);
    let equal = numeric_equal(expected, actual)?;
    let difference = absolute_difference(expected, actual)?;

    let (status, mode, tolerance_details) = match tolerance {
        None => {
            let status = if equal {
                ComparisonStatus::ExactMatch
            } else {
                ComparisonStatus::ExactMismatch
            };
            (status, ComparisonMode::Exact, None)
        }
        Some(Tolerance::Absolute { value }) => {
            let tolerance_number = parse_non_negative_tolerance(&value)?;
            let allowed_difference = NumericValue::from(tolerance_number);
            let passed = less_than_or_equal(difference, tolerance_number)?;
            let status = if passed {
                ComparisonStatus::WithinTolerance
            } else {
                ComparisonStatus::OutsideTolerance
            };
            (
                status,
                ComparisonMode::AbsoluteTolerance,
                Some(ToleranceDetails {
                    kind: ToleranceKind::Absolute,
                    value,
                    allowed_difference,
                }),
            )
        }
        Some(Tolerance::Relative { value }) => {
            let tolerance_number = parse_non_negative_tolerance(&value)?;
            let expected_magnitude = absolute_number(expected)?;
            let allowed = multiply(expected_magnitude, tolerance_number)?;
            let allowed_difference = NumericValue::from(allowed);
            let passed = less_than_or_equal(difference, allowed)?;
            let status = if passed {
                ComparisonStatus::WithinTolerance
            } else {
                ComparisonStatus::OutsideTolerance
            };
            (
                status,
                ComparisonMode::RelativeTolerance,
                Some(ToleranceDetails {
                    kind: ToleranceKind::Relative,
                    value,
                    allowed_difference,
                }),
            )
        }
    };

    let passed = matches!(
        status,
        ComparisonStatus::ExactMatch | ComparisonStatus::WithinTolerance
    );
    let difference_value = NumericValue::from(difference);
    let trace = include_trace.then(|| {
        vec![TraceStep {
            step: 1,
            operation: VERIFICATION_OPERATION_ID.to_owned(),
            inputs: vec![expected_value.clone(), actual_value.clone()],
            output: Some(difference_value.clone()),
            note: "deterministic numeric verification comparison".to_owned(),
            metadata: None,
        }]
    });

    Ok(VerificationResult {
        operation: VERIFICATION_OPERATION_ID.to_owned(),
        difference: difference_value.clone(),
        metadata: VerificationMetadata {
            engine_version: version().to_owned(),
            deterministic: true,
            assumptions: Vec::new(),
        },
        details: VerificationDetails {
            status,
            passed,
            mode,
            expected: expected_value,
            actual: actual_value,
            difference: difference_value,
            tolerance: tolerance_details,
        },
        trace: trace.unwrap_or_default(),
    })
}

fn parse_non_negative_tolerance(value: &NumericValue) -> Result<Number, ComputeError> {
    let number = value.parse_number()?;
    if is_negative(number) {
        return Err(ComputeError::invalid_input(
            "verification tolerance must be greater than or equal to zero",
        ));
    }
    Ok(number)
}

fn numeric_equal(left: Number, right: Number) -> Result<bool, ComputeError> {
    match (left, right) {
        (Number::Integer(left), Number::Integer(right)) => Ok(left == right),
        _ => {
            let (left, right) =
                aligned_decimal_coefficients(left.into_decimal(), right.into_decimal())?;
            Ok(left == right)
        }
    }
}

fn less_than_or_equal(left: Number, right: Number) -> Result<bool, ComputeError> {
    match (left, right) {
        (Number::Integer(left), Number::Integer(right)) => Ok(left <= right),
        _ => {
            let (left, right) =
                aligned_decimal_coefficients(left.into_decimal(), right.into_decimal())?;
            Ok(left <= right)
        }
    }
}

fn absolute_difference(expected: Number, actual: Number) -> Result<Number, ComputeError> {
    absolute_number(subtract(actual, expected)?)
}

fn absolute_number(value: Number) -> Result<Number, ComputeError> {
    match value {
        Number::Integer(value) => value
            .checked_abs()
            .map(Number::Integer)
            .ok_or_else(|| ComputeError::overflow("integer absolute value overflow")),
        Number::Decimal(value) => value
            .coefficient()
            .checked_abs()
            .ok_or_else(|| ComputeError::overflow("decimal absolute value overflow"))
            .and_then(|coefficient| Decimal::with_scale(coefficient, value.scale()))
            .map(Number::Decimal),
    }
}

fn is_negative(value: Number) -> bool {
    match value {
        Number::Integer(value) => value < 0,
        Number::Decimal(value) => value.coefficient() < 0,
    }
}

fn aligned_decimal_coefficients(
    left: Decimal,
    right: Decimal,
) -> Result<(i128, i128), ComputeError> {
    let scale = left.scale().max(right.scale());
    Ok((
        align_coefficient(left, scale)?,
        align_coefficient(right, scale)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{compare, ComparisonStatus, Tolerance};
    use crate::{Decimal, ErrorCode, Number, NumericValue};
    use std::str::FromStr;

    fn decimal(input: &str) -> Result<Number, crate::ComputeError> {
        Decimal::from_str(input).map(Number::Decimal)
    }

    fn tolerance_decimal(input: &str) -> Result<NumericValue, crate::ComputeError> {
        Decimal::from_str(input)
            .map(Number::Decimal)
            .map(NumericValue::from)
            .map(|value| match value {
                NumericValue::Decimal { value, scale } => NumericValue::Decimal { value, scale },
                integer => integer,
            })
    }

    #[test]
    fn exact_matches_integer_values() -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(Number::Integer(42), Number::Integer(42), None, true)?;

        assert_eq!(result.details.status, ComparisonStatus::ExactMatch);
        assert!(result.details.passed);
        assert_eq!(
            result.difference,
            NumericValue::Integer {
                value: "0".to_owned()
            }
        );
        assert_eq!(result.trace.len(), 1);
        Ok(())
    }

    #[test]
    fn exact_matches_scale_normalized_decimals() -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(decimal("1.20")?, decimal("1.2")?, None, false)?;

        assert_eq!(result.details.status, ComparisonStatus::ExactMatch);
        assert!(result.details.passed);
        assert_eq!(
            result.difference,
            NumericValue::Decimal {
                value: "0".to_owned(),
                scale: 0
            }
        );
        Ok(())
    }

    #[test]
    fn exact_mismatch_reports_absolute_difference() -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(Number::Integer(10), Number::Integer(7), None, false)?;

        assert_eq!(result.details.status, ComparisonStatus::ExactMismatch);
        assert!(!result.details.passed);
        assert_eq!(
            result.difference,
            NumericValue::Integer {
                value: "3".to_owned()
            }
        );
        Ok(())
    }

    #[test]
    fn absolute_tolerance_allows_negative_actual_difference(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(
            decimal("-10.0")?,
            decimal("-10.04")?,
            Some(Tolerance::Absolute {
                value: tolerance_decimal("0.05")?,
            }),
            false,
        )?;

        assert_eq!(result.details.status, ComparisonStatus::WithinTolerance);
        assert!(result.details.passed);
        assert_eq!(
            result.difference,
            NumericValue::Decimal {
                value: "0.04".to_owned(),
                scale: 2
            }
        );
        Ok(())
    }

    #[test]
    fn absolute_tolerance_rejects_outside_difference() -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(
            decimal("100.00")?,
            decimal("100.11")?,
            Some(Tolerance::Absolute {
                value: tolerance_decimal("0.10")?,
            }),
            false,
        )?;

        assert_eq!(result.details.status, ComparisonStatus::OutsideTolerance);
        assert!(!result.details.passed);
        Ok(())
    }

    #[test]
    fn relative_tolerance_uses_expected_magnitude() -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(
            Number::Integer(200),
            Number::Integer(202),
            Some(Tolerance::Relative {
                value: tolerance_decimal("0.01")?,
            }),
            false,
        )?;

        assert_eq!(result.details.status, ComparisonStatus::WithinTolerance);
        assert!(result.details.passed);
        let tolerance = match result.details.tolerance {
            Some(tolerance) => tolerance,
            None => return Err("expected tolerance details".into()),
        };
        assert_eq!(
            tolerance.allowed_difference,
            NumericValue::Decimal {
                value: "2".to_owned(),
                scale: 0
            }
        );
        Ok(())
    }

    #[test]
    fn relative_tolerance_with_zero_expected_requires_exact_actual(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(
            Number::Integer(0),
            Number::Integer(1),
            Some(Tolerance::Relative {
                value: tolerance_decimal("0.10")?,
            }),
            false,
        )?;

        assert_eq!(result.details.status, ComparisonStatus::OutsideTolerance);
        assert!(!result.details.passed);
        let tolerance = match result.details.tolerance {
            Some(tolerance) => tolerance,
            None => return Err("expected tolerance details".into()),
        };
        assert_eq!(
            tolerance.allowed_difference,
            NumericValue::Decimal {
                value: "0".to_owned(),
                scale: 0
            }
        );
        Ok(())
    }

    #[test]
    fn relative_tolerance_uses_negative_expected_magnitude(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(
            Number::Integer(-200),
            Number::Integer(-198),
            Some(Tolerance::Relative {
                value: tolerance_decimal("0.01")?,
            }),
            false,
        )?;

        assert_eq!(result.details.status, ComparisonStatus::WithinTolerance);
        assert!(result.details.passed);
        assert_eq!(
            result.difference,
            NumericValue::Integer {
                value: "2".to_owned()
            }
        );
        Ok(())
    }

    #[test]
    fn exact_equality_with_tolerance_reports_within_tolerance(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = compare(
            decimal("1.20")?,
            decimal("1.2")?,
            Some(Tolerance::Absolute {
                value: tolerance_decimal("0.00")?,
            }),
            false,
        )?;

        assert_eq!(result.details.status, ComparisonStatus::WithinTolerance);
        assert!(result.details.passed);
        assert_eq!(
            result.difference,
            NumericValue::Decimal {
                value: "0".to_owned(),
                scale: 0
            }
        );
        Ok(())
    }

    #[test]
    fn rejects_negative_tolerance() -> Result<(), Box<dyn std::error::Error>> {
        let error = compare(
            Number::Integer(1),
            Number::Integer(1),
            Some(Tolerance::Absolute {
                value: tolerance_decimal("-0.1")?,
            }),
            false,
        )
        .err()
        .ok_or("negative tolerance should fail")?;

        assert_eq!(error.code, ErrorCode::InvalidInput);
        Ok(())
    }

    #[test]
    fn reports_overflow_for_unrepresentable_difference() -> Result<(), Box<dyn std::error::Error>> {
        let error = compare(Number::Integer(i128::MIN), Number::Integer(0), None, false)
            .err()
            .ok_or("absolute difference should overflow")?;

        assert_eq!(error.code, ErrorCode::Overflow);
        Ok(())
    }
}
