//! Deterministic computation core.
//!
//! This crate is currently a foundation skeleton. Future workstreams will add
//! expression evaluation, unit conversion, finance calculators, verification,
//! and deterministic test-value generation.

/// Current foundation API status for downstream scaffolds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    /// Repository foundation exists, but computation features are not implemented.
    FoundationOnly,
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
    use super::{engine_status, version, EngineStatus};

    #[test]
    fn reports_foundation_status() {
        assert_eq!(engine_status(), EngineStatus::FoundationOnly);
    }

    #[test]
    fn exposes_package_version() {
        assert!(!version().is_empty());
    }
}
