use compute_core::{
    evaluate_compute_request, ComputeError, ComputeRequest, ComputeResponse, ErrorCode,
};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Usage: compute-cli [--help] [--version] [REQUEST_FILE]

Reads a JSON compute request from stdin or REQUEST_FILE and writes a JSON compute response to stdout.

Supported operations:
  arithmetic.add
  arithmetic.subtract
  arithmetic.multiply
  arithmetic.divide
  expression.evaluate
  units.convert
  finance.simple-interest
  finance.compound-interest
  finance.loan-payment
  finance.vat
  finance.percentage-change
  finance.margin-markup
  finance.cagr
  verification.compare
  test-generation.generate-expected-values
";

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Help,
    Version,
    Compute(InputSource),
}

#[derive(Debug, PartialEq, Eq)]
enum InputSource {
    Stdin,
    File(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

fn main() -> ExitCode {
    match run(env::args().skip(1), io::stdin(), io::stdout(), io::stderr()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr(), "compute-cli: {}", error.message);
            ExitCode::from(2)
        }
    }
}

fn run<I, R, W, E>(args: I, mut stdin: R, mut stdout: W, mut stderr: E) -> Result<(), CliError>
where
    I: IntoIterator<Item = String>,
    R: Read,
    W: Write,
    E: Write,
{
    match parse_args(args)? {
        CliCommand::Help => {
            stdout
                .write_all(USAGE.as_bytes())
                .map_err(|error| CliError::new(format!("failed to write help: {error}")))?;
        }
        CliCommand::Version => {
            writeln!(stdout, "{}", env!("CARGO_PKG_VERSION"))
                .map_err(|error| CliError::new(format!("failed to write version: {error}")))?;
        }
        CliCommand::Compute(source) => {
            let input = read_input(source, &mut stdin)?;
            let response = response_from_json(&input);
            write_json_response(&response, &mut stdout)?;
        }
    }

    stderr
        .flush()
        .map_err(|error| CliError::new(format!("failed to flush stderr: {error}")))?;
    stdout
        .flush()
        .map_err(|error| CliError::new(format!("failed to flush stdout: {error}")))?;
    Ok(())
}

fn parse_args<I>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(CliCommand::Compute(InputSource::Stdin));
    };

    match first.as_str() {
        "--help" | "-h" => {
            reject_extra_args(args)?;
            Ok(CliCommand::Help)
        }
        "--version" | "-V" => {
            reject_extra_args(args)?;
            Ok(CliCommand::Version)
        }
        value if value.starts_with('-') => Err(CliError::new(format!("unknown option: {value}"))),
        value => {
            reject_extra_args(args)?;
            Ok(CliCommand::Compute(InputSource::File(PathBuf::from(value))))
        }
    }
}

fn reject_extra_args<I>(mut args: I) -> Result<(), CliError>
where
    I: Iterator<Item = String>,
{
    if let Some(extra) = args.next() {
        return Err(CliError::new(format!("unexpected argument: {extra}")));
    }
    Ok(())
}

fn read_input<R>(source: InputSource, stdin: &mut R) -> Result<String, CliError>
where
    R: Read,
{
    match source {
        InputSource::Stdin => {
            let mut input = String::new();
            stdin
                .read_to_string(&mut input)
                .map_err(|error| CliError::new(format!("failed to read stdin: {error}")))?;
            Ok(input)
        }
        InputSource::File(path) => fs::read_to_string(&path)
            .map_err(|error| CliError::new(format!("failed to read {}: {error}", path.display()))),
    }
}

fn response_from_json(input: &str) -> ComputeResponse {
    let request = match serde_json::from_str::<ComputeRequest>(input) {
        Ok(request) => request,
        Err(error) => {
            return ComputeResponse::failure(invalid_input("invalid JSON request", error), None)
        }
    };

    evaluate_compute_request(request)
}

fn invalid_input(message: impl Into<String>, error: impl std::error::Error) -> ComputeError {
    ComputeError {
        code: ErrorCode::InvalidInput,
        message: message.into(),
        detail: Some(error.to_string()),
    }
}

fn write_json_response<W>(response: &ComputeResponse, writer: &mut W) -> Result<(), CliError>
where
    W: Write,
{
    serde_json::to_writer_pretty(&mut *writer, response)
        .map_err(|error| CliError::new(format!("failed to serialize response: {error}")))?;
    writeln!(writer).map_err(|error| CliError::new(format!("failed to write response: {error}")))
}

#[cfg(test)]
mod tests {
    use super::{parse_args, response_from_json, run, CliCommand, InputSource};
    use serde_json::{json, Value};
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn arithmetic_request(operation: &str, left: Value, right: Value) -> String {
        json!({
            "operation": operation,
            "input": {
                "left": left,
                "right": right
            }
        })
        .to_string()
    }

    #[test]
    fn parses_default_stdin_command() {
        assert_eq!(
            parse_args(Vec::<String>::new()),
            Ok(CliCommand::Compute(InputSource::Stdin))
        );
    }

    #[test]
    fn parses_file_command() {
        assert_eq!(
            parse_args(["request.json".to_owned()]),
            Ok(CliCommand::Compute(InputSource::File(PathBuf::from(
                "request.json"
            ))))
        );
    }

    #[test]
    fn rejects_unknown_option() {
        assert!(parse_args(["--unknown".to_owned()]).is_err());
    }

    #[test]
    fn adds_integer_operands() -> serde_json::Result<()> {
        let request = arithmetic_request(
            "arithmetic.add",
            json!({"kind": "integer", "value": "2"}),
            json!({"kind": "integer", "value": "40"}),
        );

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["operation"], "arithmetic.add");
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "integer", "value": "42"})
        );
        Ok(())
    }

    #[test]
    fn generates_expected_values() -> serde_json::Result<()> {
        let request = json!({
            "operation": "test-generation.generate-expected-values",
            "input": {
                "cases": [
                    {
                        "id": "addition",
                        "operation": "arithmetic.add",
                        "input": {
                            "left": {"kind": "integer", "value": "20"},
                            "right": {"kind": "integer", "value": "22"}
                        }
                    }
                ]
            }
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(
            response["result"]["details"]["cases"][0]["response"]["result"]["value"],
            json!({"kind": "integer", "value": "42"})
        );
        Ok(())
    }

    #[test]
    fn rejects_recursive_expected_value_generation_through_cli_path() -> serde_json::Result<()> {
        let request = json!({
            "operation": "test-generation.generate-expected-values",
            "input": {
                "cases": [
                    {
                        "id": "recursive",
                        "operation": "test-generation.generate-expected-values",
                        "input": {"cases": []}
                    }
                ]
            }
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "invalid-input");
        Ok(())
    }

    #[test]
    fn subtracts_integer_operands() -> serde_json::Result<()> {
        let request = arithmetic_request(
            "arithmetic.subtract",
            json!({"kind": "integer", "value": "50"}),
            json!({"kind": "integer", "value": "8"}),
        );

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["operation"], "arithmetic.subtract");
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "integer", "value": "42"})
        );
        Ok(())
    }

    #[test]
    fn preserves_decimal_precision_policy() -> serde_json::Result<()> {
        let request = json!({
            "operation": "arithmetic.divide",
            "input": {
                "left": {"kind": "integer", "value": "2"},
                "right": {"kind": "integer", "value": "3"}
            },
            "precision": {
                "decimalPlaces": 2,
                "rounding": "half-away-from-zero"
            }
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "decimal", "value": "0.67", "scale": 2})
        );
        assert_eq!(
            response["result"]["metadata"]["precision"]["decimalPlaces"],
            2
        );
        assert_eq!(
            response["result"]["metadata"]["precision"]["rounding"],
            "half-away-from-zero"
        );
        Ok(())
    }

    #[test]
    fn computes_finance_request_through_generic_path() -> serde_json::Result<()> {
        let request = json!({
            "operation": "finance.loan-payment",
            "input": {
                "principal": {"kind": "integer", "value": "1000"},
                "periodicRate": {"kind": "decimal", "value": "0.01", "scale": 2},
                "periods": 12
            },
            "precision": {
                "decimalPlaces": 2,
                "rounding": "half-away-from-zero"
            }
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["operation"], "finance.loan-payment");
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "decimal", "value": "88.85", "scale": 2})
        );
        assert_eq!(response["result"]["details"]["basis"], "displayed-payment");
        Ok(())
    }

    #[test]
    fn computes_unit_conversion_request_through_generic_path() -> serde_json::Result<()> {
        let request = json!({
            "operation": "units.convert",
            "input": {
                "value": {"kind": "integer", "value": "100"},
                "sourceUnit": "cm",
                "targetUnit": "m"
            },
            "precision": {
                "decimalPlaces": 2,
                "rounding": "exact"
            },
            "trace": true
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["operation"], "units.convert");
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "decimal", "value": "1.00", "scale": 2})
        );
        assert_eq!(response["result"]["details"]["dimension"], "length");
        assert_eq!(response["trace"][1]["operation"], "units.apply-factor");
        Ok(())
    }

    #[test]
    fn computes_vat_request_through_generic_path() -> serde_json::Result<()> {
        let request = json!({
            "operation": "finance.vat",
            "input": {
                "netAmount": {"kind": "integer", "value": "100"},
                "vatRate": {"kind": "decimal", "value": "0.20", "scale": 2}
            },
            "precision": {
                "decimalPlaces": 2,
                "rounding": "exact"
            }
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["operation"], "finance.vat");
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "decimal", "value": "120.00", "scale": 2})
        );
        assert_eq!(
            response["result"]["details"]["vatAmount"],
            json!({"kind": "decimal", "value": "20.00", "scale": 2})
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_vat_request_through_generic_path() -> serde_json::Result<()> {
        let request = json!({
            "operation": "finance.vat",
            "input": {
                "netAmount": {"kind": "integer", "value": "-1"},
                "vatRate": {"kind": "decimal", "value": "0.20", "scale": 2}
            }
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "invalid-input");
        Ok(())
    }

    #[test]
    fn computes_verification_request_through_generic_path() -> serde_json::Result<()> {
        let request = json!({
            "operation": "verification.compare",
            "input": {
                "expected": {"kind": "decimal", "value": "10.00", "scale": 2},
                "actual": {"kind": "decimal", "value": "10.04", "scale": 2},
                "tolerance": {
                    "kind": "absolute",
                    "value": {"kind": "decimal", "value": "0.05", "scale": 2}
                }
            },
            "trace": true
        })
        .to_string();

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["operation"], "verification.compare");
        assert_eq!(response["result"]["details"]["status"], "within-tolerance");
        assert_eq!(response["result"]["details"]["passed"], true);
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "decimal", "value": "0.04", "scale": 2})
        );
        assert_eq!(response["trace"][0]["operation"], "verification.compare");
        Ok(())
    }

    #[test]
    fn reports_compute_errors_as_json_responses() -> serde_json::Result<()> {
        let request = arithmetic_request(
            "arithmetic.divide",
            json!({"kind": "integer", "value": "1"}),
            json!({"kind": "integer", "value": "0"}),
        );

        let response = serde_json::to_value(response_from_json(&request))?;

        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "division-by-zero");
        Ok(())
    }

    #[test]
    fn malformed_json_becomes_invalid_input_response() -> serde_json::Result<()> {
        let response = serde_json::to_value(response_from_json("{"))?;

        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "invalid-input");
        Ok(())
    }

    #[test]
    fn help_writes_usage() -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        run(
            ["--help".to_owned()],
            "".as_bytes(),
            &mut stdout,
            &mut stderr,
        )
        .map_err(|error| error.message)?;

        let output = String::from_utf8(stdout)?;
        assert!(output.contains("Usage: compute-cli"));
        assert!(stderr.is_empty());
        Ok(())
    }

    #[test]
    fn version_writes_cli_version() -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        run(
            ["--version".to_owned()],
            "".as_bytes(),
            &mut stdout,
            &mut stderr,
        )
        .map_err(|error| error.message)?;

        assert_eq!(
            String::from_utf8(stdout)?,
            format!("{}\n", env!("CARGO_PKG_VERSION"))
        );
        assert!(stderr.is_empty());
        Ok(())
    }

    #[test]
    fn computes_stdin_request() -> Result<(), Box<dyn std::error::Error>> {
        let request = arithmetic_request(
            "arithmetic.multiply",
            json!({"kind": "integer", "value": "6"}),
            json!({"kind": "integer", "value": "7"}),
        );
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        run(
            Vec::<String>::new(),
            request.as_bytes(),
            &mut stdout,
            &mut stderr,
        )
        .map_err(|error| error.message)?;

        let response: Value = serde_json::from_slice(&stdout)?;
        assert_eq!(response["ok"], true);
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "integer", "value": "42"})
        );
        assert!(stderr.is_empty());
        Ok(())
    }

    #[test]
    fn computes_file_request() -> Result<(), Box<dyn std::error::Error>> {
        let request = arithmetic_request(
            "arithmetic.add",
            json!({"kind": "integer", "value": "19"}),
            json!({"kind": "integer", "value": "23"}),
        );
        let path = temporary_request_path()?;
        fs::write(&path, request)?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let result = run(
            [path.to_string_lossy().into_owned()],
            "".as_bytes(),
            &mut stdout,
            &mut stderr,
        );
        let cleanup_result = fs::remove_file(&path);

        result.map_err(|error| error.message)?;
        cleanup_result?;

        let response: Value = serde_json::from_slice(&stdout)?;
        assert_eq!(response["ok"], true);
        assert_eq!(
            response["result"]["value"],
            json!({"kind": "integer", "value": "42"})
        );
        assert!(stderr.is_empty());
        Ok(())
    }

    fn temporary_request_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let file_name = format!("compute-cli-test-{}-{timestamp}.json", std::process::id());
        Ok(env::temp_dir().join(file_name))
    }

    #[test]
    fn malformed_cli_arguments_return_runtime_error() -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = match run(
            ["one.json".to_owned(), "two.json".to_owned()],
            "".as_bytes(),
            &mut stdout,
            &mut stderr,
        ) {
            Ok(()) => return Err("extra file argument should fail".into()),
            Err(error) => error,
        };

        assert!(error.message.contains("unexpected argument"));
        assert!(stdout.is_empty());
        Ok(())
    }
}
