use hermes_core::config::{load_config_with_provider, resolve_hermes_home};
use std::{fs, path::Path};
use tempfile::tempdir;

#[test]
fn loads_realistic_hermes_config_fixture_read_only() {
    let home = tempdir().unwrap();
    let config = home.path().join("config.yaml");
    fs::write(
        &config,
        "database:\n  journal_mode: wal\nmodel:\n  default: gpt-test\n  provider: auto\n  base_url: http://localhost:8080/v1\n",
    )
    .unwrap();

    let resolved = resolve_hermes_home(Some(home.path())).unwrap();
    let loaded = load_config_with_provider(&resolved, Some("custom")).unwrap();

    assert_eq!(loaded.model.default.as_deref(), Some("gpt-test"));
    assert_eq!(loaded.model.provider.as_deref(), Some("custom"));
    assert_eq!(fs::read_to_string(config).unwrap(), "database:\n  journal_mode: wal\nmodel:\n  default: gpt-test\n  provider: auto\n  base_url: http://localhost:8080/v1\n");
    assert!(!Path::new("config.yaml.tmp").exists());
}
