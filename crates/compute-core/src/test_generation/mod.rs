//! Deterministic numeric test-case generation.

use crate::{
    evaluate_compute_request, version, ComputeError, ComputeRequest, ComputeResponse,
    ComputeResult, ErrorCode, NumericKind, NumericValue, PrecisionPolicy, ResultMetadata,
    TraceMetadata, TraceStep,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const GENERATE_EXPECTED_VALUES_OPERATION_ID: &str = "test-generation.generate-expected-values";
const MAX_CASES: usize = 100;
const MAX_CASE_ID_BYTES: usize = 128;
const MAX_CASE_INPUT_BYTES: usize = 16 * 1024;

/// Request for deterministic expected-value generation.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateExpectedValuesRequest {
    pub cases: Vec<ExpectedValueCaseSpec>,
    #[serde(default)]
    pub fail_on_case_error: bool,
    #[serde(default)]
    pub max_cases: Option<usize>,
}

/// One case whose expected value is computed through the core dispatcher.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedValueCaseSpec {
    pub id: String,
    pub operation: String,
    pub input: Value,
    #[serde(default)]
    pub precision: Option<PrecisionPolicy>,
    #[serde(default)]
    pub trace: Option<bool>,
}

/// Details returned in the generic compute result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedExpectedValuesDetails {
    pub case_count: usize,
    pub failed_case_count: usize,
    pub cases: Vec<GeneratedExpectedValueCase>,
}

/// Generated expected value for one case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedExpectedValueCase {
    pub id: String,
    pub request: ComputeRequest,
    pub response: ComputeResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FailFastCaseErrorDetail {
    case_id: String,
    case_index: usize,
    operation: String,
    error: FailFastNestedError,
    response: ComputeResponse,
    generated_case_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FailFastNestedError {
    code: ErrorCode,
    message: String,
}

/// Generates deterministic expected values by reusing supported compute operations.
pub fn generate_expected_values(
    request: GenerateExpectedValuesRequest,
    include_trace: bool,
) -> ComputeResponse {
    if let Err(error) = validate_request(&request) {
        return ComputeResponse::failure(error, None);
    }

    let mut cases = Vec::with_capacity(request.cases.len());
    let mut failed_case_count = 0;
    let mut trace = include_trace.then(Vec::new);

    for (case_index, case) in request.cases.into_iter().enumerate() {
        if case.operation == GENERATE_EXPECTED_VALUES_OPERATION_ID {
            return ComputeResponse::failure(
                ComputeError::invalid_input(
                    "test generation cases cannot recursively invoke expected-value generation",
                ),
                None,
            );
        }

        let compute_request = ComputeRequest {
            operation: case.operation,
            input: case.input,
            precision: case.precision,
            trace: case.trace.unwrap_or(false),
        };
        let response = evaluate_compute_request(compute_request.clone());
        if !response.ok {
            failed_case_count += 1;
        }

        record_case_trace(
            &mut trace,
            case_index,
            &case.id,
            &compute_request.operation,
            response.ok,
            failed_case_count,
        );

        if !response.ok && request.fail_on_case_error {
            let detail = fail_fast_detail(
                &case.id,
                case_index,
                &compute_request.operation,
                response.clone(),
                cases.len(),
            );
            return ComputeResponse::failure(
                ComputeError {
                    code: ErrorCode::InvalidInput,
                    message: format!(
                        "generated case '{}' failed: {}",
                        case.id,
                        response
                            .error
                            .as_ref()
                            .map(|error| error.message.as_str())
                            .unwrap_or("unknown error")
                    ),
                    detail,
                },
                trace,
            );
        }

        cases.push(GeneratedExpectedValueCase {
            id: case.id,
            request: compute_request,
            response,
        });
    }

    let case_count = cases.len();
    record_summary_trace(&mut trace, case_count, failed_case_count);
    let details = GeneratedExpectedValuesDetails {
        case_count,
        failed_case_count,
        cases,
    };
    let result = ComputeResult {
        operation: GENERATE_EXPECTED_VALUES_OPERATION_ID.to_owned(),
        value: NumericValue::Integer {
            value: case_count.to_string(),
        },
        metadata: ResultMetadata {
            engine_version: version().to_owned(),
            numeric_kind: NumericKind::Integer,
            precision: PrecisionPolicy::default(),
            deterministic: true,
            assumptions: vec![
                "cases are evaluated in input order".to_owned(),
                "expected values are generated by the core compute dispatcher".to_owned(),
                "no random values are generated by this operation".to_owned(),
            ],
        },
        details: serde_json::to_value(details).ok(),
    };

    ComputeResponse::success(result, trace)
}

fn validate_request(request: &GenerateExpectedValuesRequest) -> Result<(), ComputeError> {
    if request.cases.is_empty() {
        return Err(ComputeError::invalid_input(
            "expected-value generation requires at least one case",
        ));
    }

    let max_cases = request.max_cases.unwrap_or(MAX_CASES);
    if max_cases > MAX_CASES {
        return Err(ComputeError::invalid_input(format!(
            "maxCases {max_cases} exceeds maximum {MAX_CASES}"
        )));
    }
    if request.cases.len() > max_cases {
        return Err(ComputeError::invalid_input(format!(
            "case count {} exceeds maxCases {max_cases}",
            request.cases.len()
        )));
    }

    for (index, case) in request.cases.iter().enumerate() {
        if case.id.trim().is_empty() {
            return Err(ComputeError::invalid_input(format!(
                "case id at index {index} must not be empty"
            )));
        }
        if case.id.len() > MAX_CASE_ID_BYTES {
            return Err(ComputeError::invalid_input(format!(
                "case id at index {index} exceeds maximum {MAX_CASE_ID_BYTES} bytes"
            )));
        }
        if case.operation.trim().is_empty() {
            return Err(ComputeError::invalid_input(format!(
                "case '{}' operation must not be empty",
                case.id
            )));
        }
        let input_bytes = serde_json::to_vec(&case.input)
            .map_err(|error| ComputeError::invalid_input(format!("invalid case input: {error}")))?
            .len();
        if input_bytes > MAX_CASE_INPUT_BYTES {
            return Err(ComputeError::invalid_input(format!(
                "case '{}' input exceeds maximum {MAX_CASE_INPUT_BYTES} serialized bytes",
                case.id
            )));
        }
    }

    Ok(())
}

fn fail_fast_detail(
    case_id: &str,
    case_index: usize,
    operation: &str,
    response: ComputeResponse,
    generated_case_count: usize,
) -> Option<String> {
    let nested_error = response.error.as_ref().map(|error| FailFastNestedError {
        code: error.code,
        message: error.message.clone(),
    })?;
    let detail = FailFastCaseErrorDetail {
        case_id: case_id.to_owned(),
        case_index,
        operation: operation.to_owned(),
        error: nested_error,
        response,
        generated_case_count,
    };
    serde_json::to_string(&detail).ok()
}

fn record_case_trace(
    trace: &mut Option<Vec<TraceStep>>,
    case_index: usize,
    case_id: &str,
    operation: &str,
    case_ok: bool,
    failed_case_count: usize,
) {
    let Some(trace) = trace else {
        return;
    };
    trace.push(TraceStep {
        step: u32::try_from(trace.len() + 1).unwrap_or(u32::MAX),
        operation: format!("{GENERATE_EXPECTED_VALUES_OPERATION_ID}.case"),
        inputs: Vec::new(),
        output: None,
        note: "deterministic expected-value case evaluated".to_owned(),
        metadata: Some(TraceMetadata {
            evaluated_case_count: Some(case_index + 1),
            failed_case_count: Some(failed_case_count),
            case_index: Some(case_index),
            case_id: Some(case_id.to_owned()),
            case_operation: Some(operation.to_owned()),
            case_ok: Some(case_ok),
        }),
    });
}

fn record_summary_trace(
    trace: &mut Option<Vec<TraceStep>>,
    case_count: usize,
    failed_case_count: usize,
) {
    let Some(trace) = trace else {
        return;
    };
    trace.push(TraceStep {
        step: u32::try_from(trace.len() + 1).unwrap_or(u32::MAX),
        operation: GENERATE_EXPECTED_VALUES_OPERATION_ID.to_owned(),
        inputs: Vec::new(),
        output: Some(NumericValue::Integer {
            value: case_count.to_string(),
        }),
        note: "deterministic expected-value generation summary".to_owned(),
        metadata: Some(TraceMetadata {
            evaluated_case_count: Some(case_count),
            failed_case_count: Some(failed_case_count),
            case_index: None,
            case_id: None,
            case_operation: None,
            case_ok: None,
        }),
    });
}

#[cfg(test)]
mod tests {
    use super::{generate_expected_values, GenerateExpectedValuesRequest};
    use crate::{ComputeResponse, ErrorCode};
    use serde_json::{json, Value};

    fn generate(input: Value) -> serde_json::Result<ComputeResponse> {
        let request = serde_json::from_value::<GenerateExpectedValuesRequest>(input)?;
        Ok(generate_expected_values(request, false))
    }

    fn generate_with_trace(input: Value) -> serde_json::Result<ComputeResponse> {
        let request = serde_json::from_value::<GenerateExpectedValuesRequest>(input)?;
        Ok(generate_expected_values(request, true))
    }

    #[test]
    fn generates_repeatable_arithmetic_expected_values() -> serde_json::Result<()> {
        let input = json!({
            "cases": [
                {
                    "id": "integer-add",
                    "operation": "arithmetic.add",
                    "input": {
                        "left": {"kind": "integer", "value": "2"},
                        "right": {"kind": "integer", "value": "40"}
                    }
                },
                {
                    "id": "rounded-division",
                    "operation": "arithmetic.divide",
                    "input": {
                        "left": {"kind": "integer", "value": "2"},
                        "right": {"kind": "integer", "value": "3"}
                    },
                    "precision": {"decimalPlaces": 2, "rounding": "half-away-from-zero"}
                }
            ]
        });

        let first = serde_json::to_value(generate(input.clone())?)?;
        let second = serde_json::to_value(generate(input)?)?;

        assert_eq!(first, second);
        assert_eq!(first["ok"], true);
        assert_eq!(
            first["result"]["details"]["cases"][0]["response"]["result"]["value"],
            json!({"kind": "integer", "value": "42"})
        );
        assert_eq!(
            first["result"]["details"]["cases"][1]["response"]["result"]["value"],
            json!({"kind": "decimal", "value": "0.67", "scale": 2})
        );
        Ok(())
    }

    #[test]
    fn generates_expression_expected_values() -> serde_json::Result<()> {
        let response = serde_json::to_value(generate(json!({
            "cases": [
                {
                    "id": "expression",
                    "operation": "expression.evaluate",
                    "input": {"expression": "2 * (3 + 4)"}
                }
            ]
        }))?)?;

        assert_eq!(response["ok"], true);
        assert_eq!(
            response["result"]["details"]["cases"][0]["response"]["result"]["value"],
            json!({"kind": "integer", "value": "14"})
        );
        Ok(())
    }

    #[test]
    fn records_case_errors_when_not_fail_fast() -> serde_json::Result<()> {
        let response = serde_json::to_value(generate(json!({
            "cases": [
                {
                    "id": "division-by-zero",
                    "operation": "arithmetic.divide",
                    "input": {
                        "left": {"kind": "integer", "value": "1"},
                        "right": {"kind": "integer", "value": "0"}
                    }
                }
            ]
        }))?)?;

        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["details"]["failedCaseCount"], 1);
        assert_eq!(
            response["result"]["details"]["cases"][0]["response"]["error"]["code"],
            "division-by-zero"
        );
        Ok(())
    }

    #[test]
    fn fail_fast_error_includes_structured_case_detail() -> serde_json::Result<()> {
        let response = serde_json::to_value(generate(json!({
            "failOnCaseError": true,
            "cases": [
                {
                    "id": "division-by-zero",
                    "operation": "arithmetic.divide",
                    "input": {
                        "left": {"kind": "integer", "value": "1"},
                        "right": {"kind": "integer", "value": "0"}
                    }
                }
            ]
        }))?)?;
        let detail: Value = serde_json::from_str(
            response["error"]["detail"]
                .as_str()
                .unwrap_or("{\"error\":\"missing detail\"}"),
        )?;

        assert_eq!(response["ok"], false);
        assert_eq!(detail["caseId"], "division-by-zero");
        assert_eq!(detail["caseIndex"], 0);
        assert_eq!(detail["operation"], "arithmetic.divide");
        assert_eq!(detail["error"]["code"], "division-by-zero");
        assert_eq!(detail["error"]["message"], "division by zero");
        assert_eq!(detail["response"]["ok"], false);
        Ok(())
    }

    #[test]
    fn max_cases_100_succeeds_and_101_fails() -> serde_json::Result<()> {
        let cases = (0..100)
            .map(|index| {
                json!({
                    "id": format!("case-{index}"),
                    "operation": "arithmetic.add",
                    "input": {
                        "left": {"kind": "integer", "value": "1"},
                        "right": {"kind": "integer", "value": "1"}
                    }
                })
            })
            .collect::<Vec<_>>();

        let response = serde_json::to_value(generate(json!({
            "maxCases": 100,
            "cases": cases
        }))?)?;
        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["details"]["caseCount"], 100);

        let cases = (0..101)
            .map(|index| {
                json!({
                    "id": format!("case-{index}"),
                    "operation": "arithmetic.add",
                    "input": {}
                })
            })
            .collect::<Vec<_>>();
        let response = generate(json!({
            "cases": cases
        }))?;
        assert_eq!(
            response.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
        Ok(())
    }

    #[test]
    fn rejects_case_id_and_input_size_bounds() -> serde_json::Result<()> {
        let long_id = "x".repeat(129);
        let long_id_response = generate(json!({
            "cases": [
                {"id": long_id, "operation": "arithmetic.add", "input": {}}
            ]
        }))?;
        assert_eq!(
            long_id_response.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );

        let large_input = "x".repeat(16 * 1024);
        let large_input_response = generate(json!({
            "cases": [
                {
                    "id": "large-input",
                    "operation": "expression.evaluate",
                    "input": {"expression": large_input}
                }
            ]
        }))?;
        assert_eq!(
            large_input_response.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
        Ok(())
    }

    #[test]
    fn trace_metadata_includes_case_and_failure_counts() -> serde_json::Result<()> {
        let response = serde_json::to_value(generate_with_trace(json!({
            "cases": [
                {
                    "id": "ok",
                    "operation": "arithmetic.add",
                    "input": {
                        "left": {"kind": "integer", "value": "1"},
                        "right": {"kind": "integer", "value": "2"}
                    }
                },
                {
                    "id": "bad",
                    "operation": "arithmetic.divide",
                    "input": {
                        "left": {"kind": "integer", "value": "1"},
                        "right": {"kind": "integer", "value": "0"}
                    }
                }
            ]
        }))?)?;

        assert_eq!(response["trace"][0]["metadata"]["caseId"], "ok");
        assert_eq!(
            response["trace"][0]["metadata"]["caseOperation"],
            "arithmetic.add"
        );
        assert_eq!(response["trace"][0]["metadata"]["caseOk"], true);
        assert_eq!(response["trace"][1]["metadata"]["caseId"], "bad");
        assert_eq!(response["trace"][1]["metadata"]["failedCaseCount"], 1);
        assert_eq!(response["trace"][2]["metadata"]["evaluatedCaseCount"], 2);
        assert_eq!(response["trace"][2]["metadata"]["failedCaseCount"], 1);
        assert!(response["trace"][0].get("response").is_none());
        Ok(())
    }

    #[test]
    fn rejects_invalid_limits_and_recursive_cases() -> serde_json::Result<()> {
        let empty = generate(json!({"cases": []}))?;
        assert_eq!(
            empty.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );

        let too_many = generate(json!({
            "maxCases": 1,
            "cases": [
                {"id": "a", "operation": "arithmetic.add", "input": {}},
                {"id": "b", "operation": "arithmetic.add", "input": {}}
            ]
        }))?;
        assert_eq!(
            too_many.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );

        let recursive = generate(json!({
            "cases": [
                {
                    "id": "recursive",
                    "operation": "test-generation.generate-expected-values",
                    "input": {"cases": []}
                }
            ]
        }))?;
        assert_eq!(
            recursive.error.map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
        Ok(())
    }
}
