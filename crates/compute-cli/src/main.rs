use compute_core::{engine_status, version, EngineStatus};

fn main() {
    let status = match engine_status() {
        EngineStatus::FoundationOnly => "foundation-only",
    };

    println!(
        "{{\"service\":\"deterministic-compute-cli\",\"version\":\"{}\",\"status\":\"{}\"}}",
        version(),
        status
    );
}
