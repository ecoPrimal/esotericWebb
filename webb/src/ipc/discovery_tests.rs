// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for capability-based primal discovery.

use super::*;
#[cfg(unix)]
use crate::ipc::discovery_paths::uid_from_proc_status;
use crate::ipc::discovery_paths::{plasmid_bin_directories, process_uid, socket_directories};

#[test]
fn socket_directories_include_run_user_biomeos() {
    let uid = process_uid();
    let dirs = socket_directories();
    let expected = PathBuf::from(format!("/run/user/{uid}/biomeos"));
    assert!(dirs.contains(&expected));
}

#[test]
fn process_uid_returns_real_uid() {
    let uid = process_uid();
    assert_ne!(uid, u32::MAX);
}

#[test]
fn plasmidbins_include_relative_paths() {
    let dirs = plasmid_bin_directories();
    assert!(dirs.contains(&PathBuf::from("./plasmidBin")));
    assert!(dirs.contains(&PathBuf::from("../plasmidBin")));
}

#[test]
fn registry_find_missing_capability() {
    let registry = PrimalRegistry::default();
    assert!(registry.find_by_capability("dag.session.create").is_none());
}

#[test]
fn registry_find_registered_capability() {
    let mut registry = PrimalRegistry::default();
    registry.by_domain.insert(
        "dag".to_owned(),
        PrimalEndpoint {
            domain: "dag".to_owned(),
            name: "rhizocrypt".to_owned(),
            socket_path: Some(PathBuf::from("/tmp/rhizocrypt.sock")),
            tcp_addr: None,
            http_url: None,
            capabilities: vec!["dag.session.create".to_owned()],
            healthy: true,
        },
    );
    registry
        .capability_index
        .insert("dag.session.create".to_owned(), "dag".to_owned());
    assert!(registry.find_by_capability("dag.session.create").is_some());
}

#[test]
fn endpoint_can_have_both_tcp_and_uds() {
    let ep = PrimalEndpoint {
        domain: "dag".to_owned(),
        name: "rhizocrypt".to_owned(),
        socket_path: Some(PathBuf::from("/tmp/dag.sock")),
        tcp_addr: Some("127.0.0.1:9401".to_owned()),
        http_url: None,
        capabilities: vec!["dag.session.create".to_owned()],
        healthy: false,
    };
    assert!(ep.socket_path.is_some());
    assert!(ep.tcp_addr.is_some());
}

#[test]
fn metadata_ingestion_sets_tcp_and_capabilities() {
    let dir = std::env::temp_dir().join("esotericwebb-test-meta");
    let _ = std::fs::create_dir_all(&dir);
    let meta_path = dir.join("metadata.toml");
    std::fs::write(
        &meta_path,
        r#"
[primal]
name = "testprimal"

[capabilities]
domain = "testdomain"
methods = ["test.method.one", "test.method.two"]

[transport]
tcp_address = "127.0.0.1:9999"
"#,
    )
    .unwrap();

    let mut registry = PrimalRegistry::default();
    registry.ingest_metadata(&meta_path);

    let ep = registry.by_domain.get("testdomain");
    assert!(ep.is_some());
    let ep = ep.unwrap();
    assert_eq!(ep.name, "testprimal");
    assert_eq!(ep.tcp_addr.as_deref(), Some("127.0.0.1:9999"));
    assert_eq!(ep.capabilities.len(), 2);

    assert!(registry.capability_index.contains_key("test.method.one"));

    let _ = std::fs::remove_file(&meta_path);
}

#[test]
fn metadata_ingestion_port_forms_localhost() {
    let dir = std::env::temp_dir().join("esotericwebb-test-meta2");
    let _ = std::fs::create_dir_all(&dir);
    let meta_path = dir.join("metadata.toml");
    std::fs::write(
        &meta_path,
        r#"
[primal]
name = "portprimal"

[capabilities]
domain = "portdomain"
methods = ["port.test"]

[transport]
port = 9402
"#,
    )
    .unwrap();

    let mut registry = PrimalRegistry::default();
    registry.ingest_metadata(&meta_path);

    let ep = registry.by_domain.get("portdomain").unwrap();
    assert_eq!(ep.tcp_addr.as_deref(), Some("127.0.0.1:9402"));

    let _ = std::fs::remove_file(&meta_path);
}

#[test]
fn metadata_ingestion_skips_missing_primal_name() {
    let dir = std::env::temp_dir().join("esotericwebb-test-meta-noname");
    let _ = std::fs::create_dir_all(&dir);
    let meta_path = dir.join("metadata.toml");
    std::fs::write(
        &meta_path,
        r#"
[capabilities]
domain = "testdomain"
methods = ["test.method"]
"#,
    )
    .unwrap();

    let mut registry = PrimalRegistry::default();
    registry.ingest_metadata(&meta_path);
    assert!(registry.by_domain.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn metadata_ingestion_skips_missing_domain() {
    let dir = std::env::temp_dir().join("esotericwebb-test-meta-nodomain");
    let _ = std::fs::create_dir_all(&dir);
    let meta_path = dir.join("metadata.toml");
    std::fs::write(
        &meta_path,
        r#"
[primal]
name = "orphan"
"#,
    )
    .unwrap();

    let mut registry = PrimalRegistry::default();
    registry.ingest_metadata(&meta_path);
    assert!(registry.by_domain.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn metadata_ingestion_skips_bad_toml() {
    let dir = std::env::temp_dir().join("esotericwebb-test-meta-bad");
    let _ = std::fs::create_dir_all(&dir);
    let meta_path = dir.join("metadata.toml");
    std::fs::write(&meta_path, "{{{ not toml").unwrap();

    let mut registry = PrimalRegistry::default();
    registry.ingest_metadata(&meta_path);
    assert!(registry.by_domain.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn metadata_ingestion_skips_missing_file() {
    let mut registry = PrimalRegistry::default();
    registry.ingest_metadata(std::path::Path::new("/nonexistent/metadata.toml"));
    assert!(registry.by_domain.is_empty());
}

#[test]
fn probe_directory_finds_sock_files() {
    let dir = std::env::temp_dir().join("esotericwebb-test-probe");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("ai.sock"), "").unwrap();
    std::fs::write(dir.join("dag.sock"), "").unwrap();
    std::fs::write(dir.join("not_a_socket.txt"), "").unwrap();

    let mut registry = PrimalRegistry::default();
    registry.probe_directory(&dir);

    assert!(registry.by_domain.contains_key("ai"));
    assert!(registry.by_domain.contains_key("dag"));
    assert!(!registry.by_domain.contains_key("not_a_socket"));

    let ai = registry.by_domain.get("ai").unwrap();
    assert!(ai.socket_path.is_some());
    assert_eq!(ai.name, "squirrel");

    let dag = registry.by_domain.get("dag").unwrap();
    assert_eq!(dag.name, "rhizocrypt");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn probe_directory_nonexistent_is_noop() {
    let mut registry = PrimalRegistry::default();
    registry.probe_directory(std::path::Path::new("/nonexistent/probe/dir"));
    assert!(registry.by_domain.is_empty());
}

#[test]
fn probe_directory_unknown_domain_uses_filename() {
    let dir = std::env::temp_dir().join("esotericwebb-test-probe-unknown");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("custom.sock"), "").unwrap();

    let mut registry = PrimalRegistry::default();
    registry.probe_directory(&dir);

    let ep = registry.by_domain.get("custom").unwrap();
    assert_eq!(ep.name, "custom");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn probe_directory_reverse_maps_primal_slug_to_domain() {
    let dir = std::env::temp_dir().join("esotericwebb-test-probe-reverse");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("rhizocrypt.sock"), "").unwrap();
    std::fs::write(dir.join("loamspine.sock"), "").unwrap();
    std::fs::write(dir.join("toadstool.sock"), "").unwrap();

    let mut registry = PrimalRegistry::default();
    registry.probe_directory(&dir);

    let dag = registry.by_domain.get("dag").unwrap();
    assert_eq!(dag.name, "rhizocrypt");
    assert!(dag.socket_path.is_some());

    let lineage = registry.by_domain.get("lineage").unwrap();
    assert_eq!(lineage.name, "loamspine");

    let compute = registry.by_domain.get("compute").unwrap();
    assert_eq!(compute.name, "toadstool");

    assert!(!registry.by_domain.contains_key("rhizocrypt"));
    assert!(!registry.by_domain.contains_key("loamspine"));
    assert!(!registry.by_domain.contains_key("toadstool"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn probe_directory_domain_named_still_works() {
    let dir = std::env::temp_dir().join("esotericwebb-test-probe-domain");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("dag.sock"), "").unwrap();
    std::fs::write(dir.join("visualization.sock"), "").unwrap();

    let mut registry = PrimalRegistry::default();
    registry.probe_directory(&dir);

    let dag = registry.by_domain.get("dag").unwrap();
    assert_eq!(dag.name, "rhizocrypt");

    let viz = registry.by_domain.get("visualization").unwrap();
    assert_eq!(viz.name, "petaltongue");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── PrimalEndpoint resolve tests ──

#[test]
fn resolve_transport_prefers_uds() {
    let mut ep = PrimalEndpoint::empty("dag", "rhizocrypt");
    ep.socket_path = Some(PathBuf::from("/run/user/1000/biomeos/dag.sock"));
    ep.tcp_addr = Some("127.0.0.1:9401".to_owned());
    ep.healthy = true;
    let resolved = ep.resolve_transport().unwrap();
    assert_eq!(
        resolved,
        TransportEndpoint::uds("/run/user/1000/biomeos/dag.sock")
    );
}

#[test]
fn resolve_transport_falls_back_to_tcp() {
    let mut ep = PrimalEndpoint::empty("dag", "rhizocrypt");
    ep.tcp_addr = Some("127.0.0.1:9401".to_owned());
    ep.healthy = true;
    let resolved = ep.resolve_transport().unwrap();
    assert_eq!(resolved, TransportEndpoint::tcp("127.0.0.1", 9401));
}

#[test]
fn resolve_transport_none_when_empty() {
    let ep = PrimalEndpoint::empty("dag", "rhizocrypt");
    assert!(ep.resolve_transport().is_none());
}

#[test]
fn available_transports_returns_both() {
    let mut ep = PrimalEndpoint::empty("dag", "rhizocrypt");
    ep.socket_path = Some(PathBuf::from("/tmp/dag.sock"));
    ep.tcp_addr = Some("127.0.0.1:9401".to_owned());
    let transports = ep.available_transports();
    assert_eq!(transports.len(), 2);
}

#[test]
fn available_transports_empty_endpoint() {
    let ep = PrimalEndpoint::empty("dag", "rhizocrypt");
    assert!(ep.available_transports().is_empty());
}

// NOTE: discover_tcp_from_env tests omitted — set_var/remove_var
// are unsafe in edition 2024, forbidden by workspace lints.
// TCP env var discovery is exercised indirectly via integration tests.

#[test]
fn socket_directories_includes_run_user() {
    let dirs = socket_directories();
    let uid = process_uid();
    assert!(
        dirs.iter()
            .any(|d| d.to_string_lossy().contains(&format!("/run/user/{uid}")))
    );
}

#[cfg(unix)]
#[test]
fn uid_from_proc_status_returns_some() {
    let uid = uid_from_proc_status();
    assert!(uid.is_some());
}

#[test]
fn metadata_does_not_overwrite_existing_tcp_addr() {
    let dir = std::env::temp_dir().join("esotericwebb-test-meta-nooverwrite");
    let _ = std::fs::create_dir_all(&dir);
    let meta_path = dir.join("metadata.toml");
    std::fs::write(
        &meta_path,
        r#"
[primal]
name = "keeper"

[capabilities]
domain = "keepdomain"
methods = ["keep.test"]

[transport]
tcp_address = "127.0.0.1:8888"
"#,
    )
    .unwrap();

    let mut registry = PrimalRegistry::default();
    let mut existing = PrimalEndpoint::empty("keepdomain", "keeper");
    existing.tcp_addr = Some("10.0.0.1:1111".to_owned());
    registry.by_domain.insert("keepdomain".to_owned(), existing);
    registry.ingest_metadata(&meta_path);

    let ep = registry.by_domain.get("keepdomain").unwrap();
    assert_eq!(ep.tcp_addr.as_deref(), Some("10.0.0.1:1111"));

    let _ = std::fs::remove_dir_all(&dir);
}
