//! Deterministic finance and business calculators.

use crate::{
    apply_precision, version, ComputeError, Decimal, Number, NumericValue, PrecisionPolicy,
    RoundingMode,
};
use serde::{Deserialize, Serialize};

const SIMPLE_INTEREST_OPERATION_ID: &str = "finance.simple-interest";
const COMPOUND_INTEREST_OPERATION_ID: &str = "finance.compound-interest";
const LOAN_PAYMENT_OPERATION_ID: &str = "finance.loan-payment";
const PERCENTAGE_CHANGE_OPERATION_ID: &str = "finance.percentage-change";
const MARGIN_MARKUP_OPERATION_ID: &str = "finance.margin-markup";
const CAGR_OPERATION_ID: &str = "finance.cagr";
const MAX_DECIMAL_SCALE: u32 = 38;

/// Stable finance calculation result with deterministic metadata and trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceCalculationResult {
    pub operation: String,
    pub value: NumericValue,
    pub metadata: FinanceMetadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<FinanceTraceStep>,
}

/// Loan payment result with an amortization summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoanPaymentResult {
    pub operation: String,
    pub payment: NumericValue,
    pub summary: LoanAmortizationSummary,
    pub metadata: FinanceMetadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<FinanceTraceStep>,
}

/// Deterministic amortization summary for fixed-rate, end-of-period payments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoanAmortizationSummary {
    pub periods: u32,
    pub basis: LoanSummaryBasis,
    pub total_paid: NumericValue,
    pub total_interest: NumericValue,
}

/// Basis used for amortization summary totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoanSummaryBasis {
    /// Totals are computed from the payment value returned to callers.
    DisplayedPayment,
}

/// Margin and markup result for a cost/revenue pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginMarkupResult {
    pub operation: String,
    pub margin: NumericValue,
    pub markup: NumericValue,
    pub metadata: FinanceMetadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<FinanceTraceStep>,
}

/// Stable metadata emitted by finance calculators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceMetadata {
    pub operation: String,
    pub engine_version: String,
    pub deterministic: bool,
    pub precision: PrecisionPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<String>,
}

/// Deterministic trace step for finance formulas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceTraceStep {
    pub step: u32,
    pub operation: String,
    pub inputs: Vec<NumericValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<NumericValue>,
    pub note: String,
}

/// Computes simple interest as `principal * periodic_rate * periods`.
pub fn simple_interest(
    principal: Number,
    periodic_rate: Number,
    periods: u32,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<FinanceCalculationResult, ComputeError> {
    validate_non_negative(principal, "principal")?;
    validate_rate_greater_than_or_equal_to_negative_one(periodic_rate, "periodic_rate")?;

    let mut trace = FinanceTrace::new(include_trace);
    let principal_rational = number_rational(principal)?;
    let rate_rational = number_rational(periodic_rate)?;
    let periods_rational = Rational::integer(i128::from(periods));
    trace.record(
        "finance.simple-interest.inputs",
        vec![
            principal,
            periodic_rate,
            Number::Integer(i128::from(periods)),
        ],
        None,
        "principal, decimal periodic rate, and non-negative integer periods",
    );

    let interest = multiply_rationals(
        multiply_rationals(
            principal_rational,
            rate_rational,
            "simple interest overflow",
        )?,
        periods_rational,
        "simple interest overflow",
    )?;
    let value = rational_to_number(interest, precision, "simple interest")?;
    trace.record(
        "finance.simple-interest.formula",
        vec![
            principal,
            periodic_rate,
            Number::Integer(i128::from(periods)),
        ],
        Some(value),
        "interest = principal * periodic_rate * periods",
    );

    Ok(FinanceCalculationResult {
        operation: SIMPLE_INTEREST_OPERATION_ID.to_owned(),
        value: value.into(),
        metadata: metadata(
            SIMPLE_INTEREST_OPERATION_ID,
            precision,
            vec![
                "periodic_rate is a decimal rate per period, not a percentage whole number"
                    .to_owned(),
                "periods is a non-negative integer count".to_owned(),
                "simple interest is not compounded".to_owned(),
            ],
        ),
        trace: trace.steps,
    })
}

/// Computes compound future value as `principal * (1 + periodic_rate)^periods`.
pub fn compound_future_value(
    principal: Number,
    periodic_rate: Number,
    periods: u32,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<FinanceCalculationResult, ComputeError> {
    validate_non_negative(principal, "principal")?;
    validate_rate_greater_than_or_equal_to_negative_one(periodic_rate, "periodic_rate")?;

    let mut trace = FinanceTrace::new(include_trace);
    let base = add_rationals(
        Rational::integer(1),
        number_rational(periodic_rate)?,
        "compound rate overflow",
    )?;
    let factor = pow_rational(base, periods, "compound factor overflow")?;
    let future_value = multiply_rationals(
        number_rational(principal)?,
        factor,
        "compound future value overflow",
    )?;
    let value = rational_to_number(future_value, precision, "compound future value")?;
    trace.record(
        "finance.compound-interest.formula",
        vec![
            principal,
            periodic_rate,
            Number::Integer(i128::from(periods)),
        ],
        Some(value),
        "future_value = principal * (1 + periodic_rate)^periods",
    );

    Ok(FinanceCalculationResult {
        operation: COMPOUND_INTEREST_OPERATION_ID.to_owned(),
        value: value.into(),
        metadata: metadata(
            COMPOUND_INTEREST_OPERATION_ID,
            precision,
            vec![
                "periodic_rate is a decimal rate per compounding period".to_owned(),
                "periods is a non-negative integer count".to_owned(),
                "compounding occurs once per period at period end".to_owned(),
            ],
        ),
        trace: trace.steps,
    })
}

/// Computes the fixed payment for a fully amortizing loan with end-of-period payments.
pub fn loan_payment(
    principal: Number,
    periodic_rate: Number,
    periods: u32,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<LoanPaymentResult, ComputeError> {
    validate_positive(principal, "principal")?;
    validate_periods_positive(periods)?;
    validate_rate_greater_than_or_equal_to_zero(periodic_rate, "periodic_rate")?;

    let mut trace = FinanceTrace::new(include_trace);
    let principal_rational = number_rational(principal)?;
    let rate_rational = number_rational(periodic_rate)?;
    let payment_rational = if rate_rational.is_zero() {
        divide_rationals(
            principal_rational,
            Rational::integer(i128::from(periods)),
            "zero-rate loan payment overflow",
        )?
    } else {
        let base = add_rationals(Rational::integer(1), rate_rational, "loan rate overflow")?;
        let factor = pow_rational(base, periods, "loan compound factor overflow")?;
        let numerator = multiply_rationals(
            multiply_rationals(principal_rational, rate_rational, "loan numerator overflow")?,
            factor,
            "loan numerator overflow",
        )?;
        let denominator = subtract_rationals(factor, Rational::integer(1), "loan denominator")?;
        divide_rationals(numerator, denominator, "loan payment overflow")?
    };

    let payment = rational_to_number(payment_rational, precision, "loan payment")?;
    let displayed_payment_rational = number_rational(payment)?;
    let total_paid_rational = multiply_rationals(
        displayed_payment_rational,
        Rational::integer(i128::from(periods)),
        "loan total paid overflow",
    )?;
    let total_interest_rational = subtract_rationals(
        total_paid_rational,
        principal_rational,
        "loan total interest overflow",
    )?;
    let total_paid = rational_to_number(total_paid_rational, precision, "loan total paid")?;
    let total_interest =
        rational_to_number(total_interest_rational, precision, "loan total interest")?;

    trace.record(
        "finance.loan-payment.formula",
        vec![principal, periodic_rate, Number::Integer(i128::from(periods))],
        Some(payment),
        "payment = principal * periodic_rate * (1 + periodic_rate)^periods / ((1 + periodic_rate)^periods - 1); zero rate uses principal / periods",
    );

    Ok(LoanPaymentResult {
        operation: LOAN_PAYMENT_OPERATION_ID.to_owned(),
        payment: payment.into(),
        summary: LoanAmortizationSummary {
            periods,
            basis: LoanSummaryBasis::DisplayedPayment,
            total_paid: total_paid.into(),
            total_interest: total_interest.into(),
        },
        metadata: metadata(
            LOAN_PAYMENT_OPERATION_ID,
            precision,
            vec![
                "periodic_rate is a decimal rate per payment period".to_owned(),
                "payments occur at the end of each period".to_owned(),
                "payment is fixed and fully amortizes the principal over periods".to_owned(),
                "total_paid and total_interest are computed from the displayed payment".to_owned(),
                "fees, taxes, escrow, and prepayments are excluded".to_owned(),
            ],
        ),
        trace: trace.steps,
    })
}

/// Computes percentage change as `(new_value - old_value) / old_value`.
pub fn percentage_change(
    old_value: Number,
    new_value: Number,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<FinanceCalculationResult, ComputeError> {
    if number_rational(old_value)?.is_zero() {
        return Err(ComputeError::division_by_zero());
    }

    let mut trace = FinanceTrace::new(include_trace);
    let change = subtract_rationals(
        number_rational(new_value)?,
        number_rational(old_value)?,
        "percentage change numerator overflow",
    )?;
    let ratio = divide_rationals(
        change,
        number_rational(old_value)?,
        "percentage change overflow",
    )?;
    let value = rational_to_number(ratio, precision, "percentage change")?;
    trace.record(
        "finance.percentage-change.formula",
        vec![old_value, new_value],
        Some(value),
        "percentage_change = (new_value - old_value) / old_value",
    );

    Ok(FinanceCalculationResult {
        operation: PERCENTAGE_CHANGE_OPERATION_ID.to_owned(),
        value: value.into(),
        metadata: metadata(
            PERCENTAGE_CHANGE_OPERATION_ID,
            precision,
            vec![
                "result is a decimal ratio; multiply by 100 for percent points".to_owned(),
                "old_value must be non-zero".to_owned(),
            ],
        ),
        trace: trace.steps,
    })
}

/// Computes gross margin `(revenue - cost) / revenue` and markup `(revenue - cost) / cost`.
pub fn margin_markup(
    cost: Number,
    revenue: Number,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<MarginMarkupResult, ComputeError> {
    validate_positive(cost, "cost")?;
    validate_positive(revenue, "revenue")?;

    let mut trace = FinanceTrace::new(include_trace);
    let profit = subtract_rationals(
        number_rational(revenue)?,
        number_rational(cost)?,
        "profit overflow",
    )?;
    let margin = divide_rationals(profit, number_rational(revenue)?, "margin overflow")?;
    let markup = divide_rationals(profit, number_rational(cost)?, "markup overflow")?;
    let margin_value = rational_to_number(margin, precision, "margin")?;
    let markup_value = rational_to_number(markup, precision, "markup")?;
    trace.record(
        "finance.margin-markup.formula",
        vec![cost, revenue],
        Some(margin_value),
        "margin = (revenue - cost) / revenue; markup = (revenue - cost) / cost",
    );

    Ok(MarginMarkupResult {
        operation: MARGIN_MARKUP_OPERATION_ID.to_owned(),
        margin: margin_value.into(),
        markup: markup_value.into(),
        metadata: metadata(
            MARGIN_MARKUP_OPERATION_ID,
            precision,
            vec![
                "margin and markup are decimal ratios; multiply by 100 for percent points"
                    .to_owned(),
                "cost and revenue must be positive".to_owned(),
            ],
        ),
        trace: trace.steps,
    })
}

/// Computes CAGR for roots that have an exact decimal result under the precision policy.
pub fn cagr(
    beginning_value: Number,
    ending_value: Number,
    periods: u32,
    precision: PrecisionPolicy,
    include_trace: bool,
) -> Result<FinanceCalculationResult, ComputeError> {
    validate_positive(beginning_value, "beginning_value")?;
    validate_positive(ending_value, "ending_value")?;
    validate_periods_positive(periods)?;

    let ratio = divide_rationals(
        number_rational(ending_value)?,
        number_rational(beginning_value)?,
        "CAGR ratio overflow",
    )?;
    let scale = precision.decimal_places.ok_or_else(|| {
        ComputeError::precision_issue("CAGR requires an explicit decimal_places precision policy")
    })?;
    let one_plus_cagr = nth_root_rational_to_decimal(ratio, periods, scale)?;
    let cagr_value = crate::subtract(one_plus_cagr, Number::Integer(1))?;
    let value = apply_precision(cagr_value, precision)?;

    let mut trace = FinanceTrace::new(include_trace);
    trace.record(
        "finance.cagr.formula",
        vec![
            beginning_value,
            ending_value,
            Number::Integer(i128::from(periods)),
        ],
        Some(value),
        "cagr = (ending_value / beginning_value)^(1 / periods) - 1",
    );

    Ok(FinanceCalculationResult {
        operation: CAGR_OPERATION_ID.to_owned(),
        value: value.into(),
        metadata: metadata(
            CAGR_OPERATION_ID,
            precision,
            vec![
                "beginning_value and ending_value must be positive".to_owned(),
                "periods is a positive integer count".to_owned(),
                "CAGR supports only roots exactly representable at requested decimal_places"
                    .to_owned(),
            ],
        ),
        trace: trace.steps,
    })
}

fn metadata(
    operation: &str,
    precision: PrecisionPolicy,
    assumptions: Vec<String>,
) -> FinanceMetadata {
    FinanceMetadata {
        operation: operation.to_owned(),
        engine_version: version().to_owned(),
        deterministic: true,
        precision,
        assumptions,
    }
}

fn validate_non_negative(value: Number, name: &str) -> Result<(), ComputeError> {
    if compare_to_zero(value)? == OrderingToZero::Less {
        return Err(ComputeError::invalid_input(format!(
            "{name} must be non-negative"
        )));
    }
    Ok(())
}

fn validate_positive(value: Number, name: &str) -> Result<(), ComputeError> {
    if compare_to_zero(value)? != OrderingToZero::Greater {
        return Err(ComputeError::invalid_input(format!(
            "{name} must be positive"
        )));
    }
    Ok(())
}

fn validate_periods_positive(periods: u32) -> Result<(), ComputeError> {
    if periods == 0 {
        return Err(ComputeError::invalid_input("periods must be positive"));
    }
    Ok(())
}

fn validate_rate_greater_than_or_equal_to_negative_one(
    value: Number,
    name: &str,
) -> Result<(), ComputeError> {
    let rational = number_rational(value)?;
    if rational.numerator
        < rational
            .denominator
            .checked_neg()
            .ok_or_else(|| ComputeError::overflow("rate validation denominator sign overflow"))?
    {
        return Err(ComputeError::invalid_input(format!("{name} must be >= -1")));
    }
    Ok(())
}

fn validate_rate_greater_than_or_equal_to_zero(
    value: Number,
    name: &str,
) -> Result<(), ComputeError> {
    if compare_to_zero(value)? == OrderingToZero::Less {
        return Err(ComputeError::invalid_input(format!(
            "{name} must be non-negative"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderingToZero {
    Less,
    Equal,
    Greater,
}

fn compare_to_zero(value: Number) -> Result<OrderingToZero, ComputeError> {
    let rational = number_rational(value)?;
    Ok(match rational.numerator.cmp(&0) {
        std::cmp::Ordering::Less => OrderingToZero::Less,
        std::cmp::Ordering::Equal => OrderingToZero::Equal,
        std::cmp::Ordering::Greater => OrderingToZero::Greater,
    })
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

        let (numerator, denominator) = if denominator < 0 {
            (
                numerator
                    .checked_neg()
                    .ok_or_else(|| ComputeError::overflow("rational numerator sign overflow"))?,
                denominator
                    .checked_neg()
                    .ok_or_else(|| ComputeError::overflow("rational denominator sign overflow"))?,
            )
        } else {
            (numerator, denominator)
        };

        if numerator == 0 {
            return Ok(Self {
                numerator: 0,
                denominator: 1,
            });
        }

        let divisor = gcd_i128_divisor(numerator.unsigned_abs(), denominator.unsigned_abs())?;
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    fn is_zero(self) -> bool {
        self.numerator == 0
    }
}

fn number_rational(value: Number) -> Result<Rational, ComputeError> {
    match value {
        Number::Integer(value) => Ok(Rational::integer(value)),
        Number::Decimal(value) => Rational::new(value.coefficient(), pow10(value.scale())?),
    }
}

fn add_rationals(
    left: Rational,
    right: Rational,
    overflow_context: &'static str,
) -> Result<Rational, ComputeError> {
    let divisor = gcd_i128_divisor(
        left.denominator.unsigned_abs(),
        right.denominator.unsigned_abs(),
    )?;
    let left_multiplier = checked_div(right.denominator, divisor, overflow_context)?;
    let right_multiplier = checked_div(left.denominator, divisor, overflow_context)?;
    let left_numerator = left
        .numerator
        .checked_mul(left_multiplier)
        .ok_or_else(|| ComputeError::overflow(overflow_context))?;
    let right_numerator = right
        .numerator
        .checked_mul(right_multiplier)
        .ok_or_else(|| ComputeError::overflow(overflow_context))?;
    let numerator = left_numerator
        .checked_add(right_numerator)
        .ok_or_else(|| ComputeError::overflow(overflow_context))?;
    let denominator = left
        .denominator
        .checked_mul(left_multiplier)
        .ok_or_else(|| ComputeError::overflow(overflow_context))?;
    Rational::new(numerator, denominator)
}

fn subtract_rationals(
    left: Rational,
    right: Rational,
    overflow_context: &'static str,
) -> Result<Rational, ComputeError> {
    add_rationals(
        left,
        Rational::new(
            right
                .numerator
                .checked_neg()
                .ok_or_else(|| ComputeError::overflow(overflow_context))?,
            right.denominator,
        )?,
        overflow_context,
    )
}

fn multiply_rationals(
    left: Rational,
    right: Rational,
    overflow_context: &'static str,
) -> Result<Rational, ComputeError> {
    let mut left_numerator = left.numerator;
    let mut right_numerator = right.numerator;
    let mut left_denominator = left.denominator;
    let mut right_denominator = right.denominator;

    reduce_pair(&mut left_numerator, &mut right_denominator)?;
    reduce_pair(&mut right_numerator, &mut left_denominator)?;

    let numerator = left_numerator
        .checked_mul(right_numerator)
        .ok_or_else(|| ComputeError::overflow(overflow_context))?;
    let denominator = left_denominator
        .checked_mul(right_denominator)
        .ok_or_else(|| ComputeError::overflow(overflow_context))?;
    Rational::new(numerator, denominator)
}

fn divide_rationals(
    left: Rational,
    right: Rational,
    overflow_context: &'static str,
) -> Result<Rational, ComputeError> {
    if right.numerator == 0 {
        return Err(ComputeError::division_by_zero());
    }
    multiply_rationals(
        left,
        Rational::new(right.denominator, right.numerator)?,
        overflow_context,
    )
}

fn pow_rational(
    base: Rational,
    exponent: u32,
    overflow_context: &'static str,
) -> Result<Rational, ComputeError> {
    let mut result = Rational::integer(1);
    let mut factor = base;
    let mut remaining = exponent;
    while remaining > 0 {
        if remaining % 2 == 1 {
            result = multiply_rationals(result, factor, overflow_context)?;
        }
        remaining /= 2;
        if remaining > 0 {
            factor = multiply_rationals(factor, factor, overflow_context)?;
        }
    }
    Ok(result)
}

fn rational_to_number(
    rational: Rational,
    precision: PrecisionPolicy,
    context: &'static str,
) -> Result<Number, ComputeError> {
    if let Some(decimal_places) = precision.decimal_places {
        let quotient = scaled_divide_and_round(
            rational.numerator,
            rational.denominator,
            pow10(decimal_places)?,
            precision.rounding,
            context,
        )?;
        return Decimal::with_scale(quotient, decimal_places).map(Number::Decimal);
    }

    let extra_scale = terminating_scale(rational.denominator)?;
    let quotient = scaled_divide_and_round(
        rational.numerator,
        rational.denominator,
        pow10(extra_scale)?,
        RoundingMode::Exact,
        context,
    )?;
    Decimal::new(quotient, extra_scale).map(Number::Decimal)
}

fn nth_root_rational_to_decimal(
    rational: Rational,
    root: u32,
    decimal_places: u32,
) -> Result<Number, ComputeError> {
    if decimal_places > MAX_DECIMAL_SCALE {
        return Err(ComputeError::precision_issue(format!(
            "decimal scale {decimal_places} exceeds maximum {MAX_DECIMAL_SCALE}"
        )));
    }

    let numerator_root = exact_nth_root_i128(rational.numerator, root)?;
    let denominator_root = exact_nth_root_i128(rational.denominator, root)?;
    rational_to_number(
        Rational::new(numerator_root, denominator_root)?,
        PrecisionPolicy {
            decimal_places: Some(decimal_places),
            rounding: RoundingMode::Exact,
        },
        "CAGR exact root",
    )
}

fn exact_nth_root_i128(value: i128, root: u32) -> Result<i128, ComputeError> {
    if value < 0 {
        return Err(ComputeError::invalid_input(
            "CAGR root target must be non-negative",
        ));
    }

    if root == 1 {
        return Ok(value);
    }

    let mut low = 0_i128;
    let mut high = value;
    while low <= high {
        let mid = low + ((high - low) / 2);
        match compare_power_to(mid, root, value)? {
            std::cmp::Ordering::Equal => return Ok(mid),
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid - 1,
        }
    }
    Err(ComputeError::precision_issue(
        "CAGR root is not exactly representable at requested precision",
    ))
}

fn compare_power_to(
    base: i128,
    exponent: u32,
    target: i128,
) -> Result<std::cmp::Ordering, ComputeError> {
    let mut value = 1_i128;
    for _ in 0..exponent {
        value = match value.checked_mul(base) {
            Some(value) if value <= target => value,
            Some(_) => return Ok(std::cmp::Ordering::Greater),
            None => return Ok(std::cmp::Ordering::Greater),
        };
    }
    Ok(value.cmp(&target))
}

fn scaled_divide_and_round(
    numerator: i128,
    denominator: i128,
    scale: i128,
    rounding: RoundingMode,
    context: &'static str,
) -> Result<i128, ComputeError> {
    let mut reduced_scale = scale;
    let mut reduced_denominator = denominator;
    reduce_pair(&mut reduced_scale, &mut reduced_denominator)?;

    if let Some(scaled_numerator) = numerator.checked_mul(reduced_scale) {
        return divide_and_round(scaled_numerator, reduced_denominator, rounding, context);
    }

    let quotient = checked_div(numerator, reduced_denominator, context)?;
    let remainder = checked_rem(numerator, reduced_denominator, context)?;
    let scaled_quotient = quotient
        .checked_mul(reduced_scale)
        .ok_or_else(|| ComputeError::overflow(context))?;
    let scaled_remainder = scaled_remainder_quotient(
        remainder,
        reduced_denominator,
        reduced_scale,
        rounding,
        context,
    )?;
    scaled_quotient
        .checked_add(scaled_remainder)
        .ok_or_else(|| ComputeError::overflow(context))
}

fn scaled_remainder_quotient(
    remainder: i128,
    denominator: i128,
    scale: i128,
    rounding: RoundingMode,
    context: &'static str,
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
        .ok_or_else(|| ComputeError::overflow(context))?;
    divide_and_round(scaled_remainder, reduced_denominator, rounding, context)
}

fn divide_and_round(
    numerator: i128,
    denominator: i128,
    rounding: RoundingMode,
    context: &'static str,
) -> Result<i128, ComputeError> {
    let quotient = checked_div(numerator, denominator, context)?;
    let remainder = checked_rem(numerator, denominator, context)?;
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
                    .ok_or_else(|| ComputeError::overflow(context))
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
        return Err(ComputeError::repeating_decimal_expansion());
    }

    Ok(twos.max(fives))
}

fn reduce_pair(numerator: &mut i128, denominator: &mut i128) -> Result<(), ComputeError> {
    let divisor = gcd_i128_divisor(numerator.unsigned_abs(), denominator.unsigned_abs())?;
    if divisor > 1 {
        *numerator /= divisor;
        *denominator /= divisor;
    }
    Ok(())
}

fn gcd_i128_divisor(left: u128, right: u128) -> Result<i128, ComputeError> {
    let divisor = gcd(left, right);
    i128::try_from(divisor)
        .map_err(|_| ComputeError::overflow("finance fraction reduction divisor overflow"))
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn checked_div(
    numerator: i128,
    denominator: i128,
    context: &'static str,
) -> Result<i128, ComputeError> {
    numerator
        .checked_div(denominator)
        .ok_or_else(|| ComputeError::overflow(context))
}

fn checked_rem(
    numerator: i128,
    denominator: i128,
    context: &'static str,
) -> Result<i128, ComputeError> {
    numerator
        .checked_rem(denominator)
        .ok_or_else(|| ComputeError::overflow(context))
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
            .ok_or_else(|| ComputeError::overflow("finance power-of-ten overflow"))?;
    }
    Ok(value)
}

struct FinanceTrace {
    enabled: bool,
    steps: Vec<FinanceTraceStep>,
}

impl FinanceTrace {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            steps: Vec::new(),
        }
    }

    fn record(&mut self, operation: &str, inputs: Vec<Number>, output: Option<Number>, note: &str) {
        if !self.enabled {
            return;
        }

        self.steps.push(FinanceTraceStep {
            step: u32::try_from(self.steps.len() + 1).unwrap_or(u32::MAX),
            operation: operation.to_owned(),
            inputs: inputs.into_iter().map(NumericValue::from).collect(),
            output: output.map(NumericValue::from),
            note: note.to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorCode, RoundingMode};
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

    fn decimal(input: &str) -> Result<Number, ComputeError> {
        Decimal::from_str(input).map(Number::Decimal)
    }

    fn decimal_with_scale(coefficient: &str, scale: u32) -> Result<Number, ComputeError> {
        let coefficient = coefficient
            .parse::<i128>()
            .map_err(|_| ComputeError::invalid_input("invalid test coefficient"))?;
        Decimal::with_scale(coefficient, scale).map(Number::Decimal)
    }

    #[test]
    fn computes_exact_simple_interest() -> Result<(), ComputeError> {
        let result = simple_interest(
            Number::Integer(1000),
            decimal("0.05")?,
            3,
            PrecisionPolicy::default(),
            true,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "150".to_owned(),
                scale: 0,
            }
        );
        assert_eq!(result.metadata.operation, SIMPLE_INTEREST_OPERATION_ID);
        assert_eq!(result.trace.len(), 2);
        assert_eq!(result.trace[1].step, 2);
        Ok(())
    }

    #[test]
    fn compounds_future_value_with_rounding() -> Result<(), ComputeError> {
        let result = compound_future_value(
            Number::Integer(1000),
            decimal("0.05")?,
            2,
            round_scale(2),
            true,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "1102.50".to_owned(),
                scale: 2,
            }
        );
        assert_eq!(
            result.trace[0].operation,
            "finance.compound-interest.formula"
        );
        Ok(())
    }

    #[test]
    fn computes_zero_rate_loan_summary() -> Result<(), ComputeError> {
        let result = loan_payment(
            Number::Integer(1200),
            Number::Integer(0),
            12,
            exact_scale(2),
            true,
        )?;

        assert_eq!(
            result.payment,
            NumericValue::Decimal {
                value: "100.00".to_owned(),
                scale: 2,
            }
        );
        assert_eq!(
            result.summary.total_interest,
            NumericValue::Decimal {
                value: "0.00".to_owned(),
                scale: 2,
            }
        );
        assert_eq!(result.summary.basis, LoanSummaryBasis::DisplayedPayment);
        assert_eq!(
            result.metadata.assumptions[1],
            "payments occur at the end of each period"
        );
        Ok(())
    }

    #[test]
    fn computes_rounded_loan_payment() -> Result<(), ComputeError> {
        let result = loan_payment(
            Number::Integer(1000),
            decimal("0.01")?,
            12,
            round_scale(2),
            false,
        )?;

        assert_eq!(
            result.payment,
            NumericValue::Decimal {
                value: "88.85".to_owned(),
                scale: 2,
            }
        );
        assert_eq!(
            result.summary.total_paid,
            NumericValue::Decimal {
                value: "1066.20".to_owned(),
                scale: 2,
            }
        );
        assert_eq!(
            result.summary.total_interest,
            NumericValue::Decimal {
                value: "66.20".to_owned(),
                scale: 2,
            }
        );
        assert!(result.trace.is_empty());
        Ok(())
    }

    #[test]
    fn computes_percentage_change_as_ratio() -> Result<(), ComputeError> {
        let result = percentage_change(
            Number::Integer(80),
            Number::Integer(100),
            exact_scale(2),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "0.25".to_owned(),
                scale: 2,
            }
        );
        assert!(result
            .metadata
            .assumptions
            .iter()
            .any(|assumption| assumption.contains("decimal ratio")));
        Ok(())
    }

    #[test]
    fn computes_negative_percentage_change() -> Result<(), ComputeError> {
        let result = percentage_change(
            Number::Integer(100),
            Number::Integer(80),
            exact_scale(2),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "-0.20".to_owned(),
                scale: 2,
            }
        );
        Ok(())
    }

    #[test]
    fn computes_margin_and_markup() -> Result<(), ComputeError> {
        let result = margin_markup(
            Number::Integer(60),
            Number::Integer(100),
            round_scale(4),
            true,
        )?;

        assert_eq!(
            result.margin,
            NumericValue::Decimal {
                value: "0.4000".to_owned(),
                scale: 4,
            }
        );
        assert_eq!(
            result.markup,
            NumericValue::Decimal {
                value: "0.6667".to_owned(),
                scale: 4,
            }
        );
        assert_eq!(result.trace[0].step, 1);
        Ok(())
    }

    #[test]
    fn computes_exact_cagr_when_root_is_representable() -> Result<(), ComputeError> {
        let result = cagr(
            Number::Integer(100),
            Number::Integer(121),
            2,
            exact_scale(2),
            true,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "0.10".to_owned(),
                scale: 2,
            }
        );
        assert_eq!(result.trace[0].operation, "finance.cagr.formula");
        Ok(())
    }

    #[test]
    fn computes_high_scale_exact_cagr_without_scale_power_overflow() -> Result<(), ComputeError> {
        let result = cagr(
            Number::Integer(1),
            decimal_with_scale("1000000000000000002000000000000000001", 36)?,
            2,
            exact_scale(18),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "0.000000000000000001".to_owned(),
                scale: 18,
            }
        );
        Ok(())
    }

    #[test]
    fn computes_exact_cagr_root_above_previous_search_cap() -> Result<(), ComputeError> {
        let result = cagr(
            Number::Integer(1),
            Number::Integer(100000000000000000000000000000000000000),
            2,
            exact_scale(0),
            false,
        )?;

        assert_eq!(
            result.value,
            NumericValue::Decimal {
                value: "9999999999999999999".to_owned(),
                scale: 0,
            }
        );
        Ok(())
    }

    #[test]
    fn rejects_non_exact_cagr_root_with_precision_error() {
        let error = cagr(
            Number::Integer(100),
            Number::Integer(200),
            2,
            exact_scale(2),
            false,
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.code),
            Some(ErrorCode::PrecisionIssue)
        );
        assert!(error
            .as_ref()
            .is_some_and(|error| error.message.contains("CAGR root")));
    }

    #[test]
    fn rejects_cagr_without_explicit_precision() {
        let error = cagr(
            Number::Integer(100),
            Number::Integer(121),
            2,
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
    fn rejects_invalid_inputs() -> Result<(), ComputeError> {
        let negative_principal = simple_interest(
            Number::Integer(-1),
            decimal("0.05")?,
            1,
            PrecisionPolicy::default(),
            false,
        )
        .err();
        let zero_period_loan = loan_payment(
            Number::Integer(100),
            Number::Integer(0),
            0,
            exact_scale(2),
            false,
        )
        .err();
        let zero_old_value = percentage_change(
            Number::Integer(0),
            Number::Integer(1),
            exact_scale(2),
            false,
        )
        .err();

        assert_eq!(
            negative_principal.as_ref().map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
        assert_eq!(
            zero_period_loan.as_ref().map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
        assert_eq!(
            zero_old_value.as_ref().map(|error| error.code),
            Some(ErrorCode::DivisionByZero)
        );
        Ok(())
    }

    #[test]
    fn handles_negative_one_rate_and_zero_principal_edges() -> Result<(), ComputeError> {
        let zero_future_value = compound_future_value(
            Number::Integer(100),
            Number::Integer(-1),
            2,
            exact_scale(0),
            false,
        )?;
        let zero_principal_future_value = compound_future_value(
            Number::Integer(0),
            decimal("0.25")?,
            3,
            exact_scale(2),
            false,
        )?;

        assert_eq!(
            zero_future_value.value,
            NumericValue::Decimal {
                value: "0".to_owned(),
                scale: 0,
            }
        );
        assert_eq!(
            zero_principal_future_value.value,
            NumericValue::Decimal {
                value: "0.00".to_owned(),
                scale: 2,
            }
        );
        Ok(())
    }

    #[test]
    fn reports_overflow_for_large_compound_and_loan_inputs() {
        let compound_error = compound_future_value(
            Number::Integer(i128::MAX),
            Number::Integer(1),
            2,
            PrecisionPolicy::default(),
            false,
        )
        .err();
        let loan_error = loan_payment(
            Number::Integer(i128::MAX),
            Number::Integer(1),
            2,
            PrecisionPolicy::default(),
            false,
        )
        .err();

        assert_eq!(
            compound_error.as_ref().map(|error| error.code),
            Some(ErrorCode::Overflow)
        );
        assert_eq!(
            loan_error.as_ref().map(|error| error.code),
            Some(ErrorCode::Overflow)
        );
    }

    #[test]
    fn trace_is_deterministic() -> Result<(), ComputeError> {
        let left = compound_future_value(
            Number::Integer(1000),
            decimal("0.05")?,
            2,
            round_scale(2),
            true,
        )?;
        let right = compound_future_value(
            Number::Integer(1000),
            decimal("0.05")?,
            2,
            round_scale(2),
            true,
        )?;

        assert_eq!(left.trace, right.trace);
        assert_eq!(left.metadata, right.metadata);
        Ok(())
    }

    #[test]
    fn exact_precision_rejects_repeating_decimal() -> Result<(), ComputeError> {
        let error = loan_payment(
            Number::Integer(1000),
            decimal("0.01")?,
            12,
            exact_scale(2),
            false,
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.code),
            Some(ErrorCode::PrecisionIssue)
        );
        Ok(())
    }
}
