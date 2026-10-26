//! rust-ddd architecture tests. `checker.rs` is the standard; this file is the project's configuration.

mod checker;

use std::sync::OnceLock;

use checker::{Config, Layer, Variant, Violation};

fn config() -> Config {
    Config {
        variant: Variant::Small,
        root: env!("CARGO_MANIFEST_DIR").into(),
        // One context: the four layers sit directly under `src/`.
        contexts: vec![],
        kernel: "kernel",
        domain_allowed: vec![
            "kernel",
            "serde",
            "thiserror",
            "chrono",
            "uuid",
            "async_trait",
            "tracing",
            "rust_decimal",
        ],
        denied: vec![
            (Layer::Domain, "crate::kernel::http"),
            (Layer::Application, "crate::kernel::http"),
            (Layer::Application, "sqlx"),
            (Layer::Application, "axum"),
            (Layer::Application, "reqwest"),
            (Layer::Application, "clickhouse"),
            (Layer::Application, "redis"),
            (Layer::Application, "anyhow"),
            (Layer::Application, "tokio::fs"),
            (Layer::Application, "tokio::io"),
            (Layer::Application, "tokio::net"),
            (Layer::Application, "tokio::process"),
            (Layer::Infrastructure, "anyhow"),
            (Layer::Presentation, "sqlx"),
            (Layer::Presentation, "reqwest"),
            (Layer::Presentation, "clickhouse"),
            (Layer::Presentation, "redis"),
            (Layer::Presentation, "anyhow"),
        ],
        tech_prefixes: vec!["Pg", "Ch", "Redis", "InMemory", "Fs", "S3", "Cached"],
        non_contexts: vec![],
    }
}

fn violations() -> &'static [Violation] {
    static FOUND: OnceLock<Vec<Violation>> = OnceLock::new();
    FOUND.get_or_init(|| checker::check(&config()))
}

fn assert_clean(rule: &str) {
    let found: Vec<String> = violations()
        .iter()
        .filter(|v| v.rule == rule)
        .map(ToString::to_string)
        .collect();
    assert!(found.is_empty(), "{rule}:\n{}", found.join("\n"));
}

#[test]
fn a01_context_crate_root_exposes_published_and_wiring_only() {
    assert_clean("A-01");
}

#[test]
fn a02_layers_point_inward() {
    assert_clean("A-02");
}

#[test]
fn a03_contexts_meet_only_through_published_and_acl() {
    assert_clean("A-03");
}

#[test]
fn a04_dependency_allow_and_deny_lists() {
    assert_clean("A-04");
}

#[test]
fn a05_domain_is_deterministic_and_nothing_reads_env() {
    assert_clean("A-05");
}

#[test]
fn a06_module_names() {
    assert_clean("A-06");
}

#[test]
fn a07_no_mod_rs() {
    assert_clean("A-07");
}

#[test]
fn a08_message_handlers_drive_their_own_use_case() {
    assert_clean("A-08");
}

#[test]
fn a09_app_services_and_repositories_stay_behind_use_cases() {
    assert_clean("A-09");
}

#[test]
fn a10_banned_type_name_suffixes() {
    assert_clean("A-10");
}

#[test]
fn a11_adapters_carry_a_tech_prefix_or_acl_suffix() {
    assert_clean("A-11");
}

#[test]
fn a12_domain_state_changes_through_behavior_only() {
    assert_clean("A-12");
}

#[test]
fn a13_aggregate_ids_are_never_optional() {
    assert_clean("A-13");
}

#[test]
fn a14_domain_events_are_must_use() {
    assert_clean("A-14");
}
