//! Tests for slot discovery and selection — the logic that decides *which* daemon on a host an
//! `install`/`uninstall` command acts on. The filesystem side is exercised against a temp config
//! base; the decision side is a pure function of what was discovered, so it needs neither a
//! terminal nor root.

use super::*;

/// Write a daemon `config.json` for `slot` under `base`, as an installed daemon would have after
/// its handshake (server-assigned name and id cached, plus the key it authenticates with).
fn write_install(base: &Path, slot: &str, name: Option<&str>, key: &str) -> Uuid {
    let id = Uuid::new_v4();
    let dir = base.join(slot);
    std::fs::create_dir_all(&dir).unwrap();
    let config = serde_json::json!({
        "name": name,
        "id": name.map(|_| id.to_string()),
        "daemon_api_key": key,
    });
    std::fs::write(
        dir.join("config.json"),
        serde_json::to_string(&config).unwrap(),
    )
    .unwrap();
    id
}

#[test]
fn discovers_every_install_under_a_config_base() {
    let base = tempfile::tempdir().unwrap();
    let edge_id = write_install(base.path(), DEFAULT_NAME, Some("edge-01"), "key-a");
    write_install(base.path(), "scanopy-daemon-2", Some("edge-02"), "key-b");
    // A daemon installed but not yet connected has no server-assigned name or id yet.
    write_install(base.path(), "scanopy-daemon-3", None, "key-c");
    // Not an install: a stray directory with no config.
    std::fs::create_dir_all(base.path().join("logs")).unwrap();

    let found = installed_in(base.path());

    assert_eq!(found.len(), 3, "found {found:?}");
    let edge = found.iter().find(|i| i.slot == DEFAULT_NAME).unwrap();
    assert_eq!(edge.name.as_deref(), Some("edge-01"));
    assert_eq!(edge.daemon_id, Some(edge_id));
    assert_eq!(edge.api_key.as_deref(), Some("key-a"));

    let unconnected = found.iter().find(|i| i.slot == "scanopy-daemon-3").unwrap();
    assert_eq!(unconnected.name, None);
    assert_eq!(unconnected.daemon_id, None);
}

#[test]
fn an_install_is_selectable_by_name_slot_service_id_or_daemon_id() {
    let base = tempfile::tempdir().unwrap();
    let id = write_install(base.path(), "scanopy-daemon-2", Some("edge-02"), "key-b");
    let found = installed_in(base.path());
    let entry = &found[0];

    for selector in [
        "edge-02",
        "EDGE-02",
        "scanopy-daemon-2",
        &id.to_string(),
        &id.simple().to_string(),
    ] {
        assert!(entry.matches(selector), "{selector} should select edge-02");
    }
    assert!(!entry.matches("edge-01"));
}

/// The core of the multi-instance regression: with a daemon already installed, a command whose key
/// matches nothing must never resolve onto that existing install. Overwriting it is what silently
/// destroyed the first daemon.
#[test]
fn an_unrecognised_key_never_resolves_onto_an_existing_install() {
    let base = tempfile::tempdir().unwrap();
    write_install(base.path(), DEFAULT_NAME, Some("edge-01"), "key-a");
    let installed = installed_in(base.path());

    let resolution = resolve_install_target(&installed, None, None, Some("key-b")).unwrap();

    assert_eq!(resolution, Resolution::Ambiguous);
}

#[test]
fn resolution_prefers_an_explicit_selector_then_a_matching_key() {
    let base = tempfile::tempdir().unwrap();
    write_install(base.path(), DEFAULT_NAME, Some("edge-01"), "key-a");
    write_install(base.path(), "scanopy-daemon-2", Some("edge-02"), "key-b");
    let installed = installed_in(base.path());
    let index_of = |name: &str| {
        installed
            .iter()
            .position(|i| i.name.as_deref() == Some(name))
    };

    // An explicit selector wins even when another install holds the key being installed.
    let by_selector =
        resolve_install_target(&installed, Some("edge-02"), None, Some("key-a")).unwrap();
    assert_eq!(
        by_selector,
        Resolution::Resolved(InstallTarget::Existing(index_of("edge-02").unwrap()))
    );

    // Re-running an install command against the daemon that already holds that key updates it in
    // place rather than allocating a second slot.
    let by_key = resolve_install_target(&installed, None, None, Some("key-a")).unwrap();
    assert_eq!(
        by_key,
        Resolution::Resolved(InstallTarget::Existing(index_of("edge-01").unwrap()))
    );
}

/// Installs predating slots were namespaced by `--name`, and provisioning scripts still pass it.
/// While such an install exists, a command carrying its name has to keep landing on it — otherwise
/// rotating that daemon's key would quietly leave a duplicate install behind. A name matching no
/// install carries no such meaning and must not hijack the resolution.
#[test]
fn a_name_matching_an_install_from_before_slots_still_targets_it() {
    let base = tempfile::tempdir().unwrap();
    write_install(base.path(), "eth0", Some("edge-01"), "key-a");
    let installed = installed_in(base.path());

    let rekeyed =
        resolve_install_target(&installed, None, Some("eth0"), Some("rotated-key")).unwrap();
    assert_eq!(
        rekeyed,
        Resolution::Resolved(InstallTarget::Existing(0)),
        "re-keying the eth0 install must update it, not add a second one"
    );

    let unrelated =
        resolve_install_target(&installed, None, Some("iot"), Some("rotated-key")).unwrap();
    assert_eq!(unrelated, Resolution::Ambiguous);
}

#[test]
fn a_first_install_takes_the_default_slot() {
    let resolution = resolve_install_target(&[], None, None, Some("key-a")).unwrap();

    assert_eq!(
        resolution,
        Resolution::Resolved(InstallTarget::New(DEFAULT_NAME.to_string()))
    );
}

#[test]
fn an_unknown_selector_is_an_error_that_names_what_is_installed() {
    let base = tempfile::tempdir().unwrap();
    write_install(base.path(), DEFAULT_NAME, Some("edge-01"), "key-a");
    let installed = installed_in(base.path());

    let error = resolve_install_target(&installed, Some("edge-99"), None, Some("key-a"))
        .expect_err("an unknown selector cannot resolve");

    let message = format!("{error:#}");
    assert!(message.contains("edge-99"), "{message}");
    assert!(message.contains("edge-01"), "{message}");
}

/// The regression check: a second daemon installed on a host that already runs one must get its
/// own service and its own config directory, or installing it destroys the first.
#[test]
fn extra_daemons_get_their_own_service_and_config_directory() {
    let base = tempfile::tempdir().unwrap();
    write_install(base.path(), DEFAULT_NAME, Some("edge-01"), "key-a");
    let installed = installed_in(base.path());

    let second = next_free_slot(&installed);
    assert_ne!(second, DEFAULT_NAME);
    assert_ne!(service_id(&second), service_id(DEFAULT_NAME));
    assert_ne!(system_config_dir(&second), system_config_dir(DEFAULT_NAME));

    // And a third, once the second is taken.
    write_install(base.path(), &second, Some("edge-02"), "key-b");
    let installed = installed_in(base.path());
    let third = next_free_slot(&installed);
    assert!(![DEFAULT_NAME, second.as_str()].contains(&third.as_str()));
    assert_ne!(service_id(&third), service_id(&second));
    assert_ne!(system_config_dir(&third), system_config_dir(&second));
}

/// Daemons installed before slots existed were registered under a service id derived from their
/// `--name`. That derivation has to keep producing the same id, or those services become
/// unreachable to `uninstall`.
#[test]
fn installs_predating_slots_keep_the_service_id_they_were_registered_under() {
    assert_eq!(service_id("edge-01"), "scanopy-daemon-edge-01");
    assert_eq!(
        service_id("scanopy-daemon-home-network"),
        "scanopy-daemon-scanopy-daemon-home-network"
    );
    // …while an allocated slot is its own service id rather than doubling the prefix.
    assert_eq!(service_id("scanopy-daemon-2"), "scanopy-daemon-2");
}

fn uninstall_args(name: Option<&str>, all: bool) -> UninstallArgs {
    UninstallArgs {
        name: name.map(str::to_string),
        all,
        purge: false,
    }
}

#[test]
fn uninstall_resolves_the_only_install_without_being_told() {
    let base = tempfile::tempdir().unwrap();
    write_install(base.path(), "scanopy-daemon-2", Some("edge-02"), "key-b");
    let installed = installed_in(base.path());

    let targets = uninstall_targets(&installed, &uninstall_args(None, false)).unwrap();

    assert_eq!(targets, vec!["scanopy-daemon-2".to_string()]);
}

#[test]
fn uninstall_selects_by_daemon_name_and_removes_all_on_request() {
    let base = tempfile::tempdir().unwrap();
    write_install(base.path(), DEFAULT_NAME, Some("edge-01"), "key-a");
    write_install(base.path(), "scanopy-daemon-2", Some("edge-02"), "key-b");
    let installed = installed_in(base.path());

    let one = uninstall_targets(&installed, &uninstall_args(Some("edge-02"), false)).unwrap();
    assert_eq!(one, vec!["scanopy-daemon-2".to_string()]);

    let every = uninstall_targets(&installed, &uninstall_args(None, true)).unwrap();
    assert_eq!(every.len(), 2);
    assert!(every.contains(&DEFAULT_NAME.to_string()));
}

/// A half-removed install (service still registered, config already gone) is invisible to
/// discovery, so an explicit selector has to fall through to the paths for that name — otherwise
/// there is no way left to deregister the service.
#[test]
fn uninstall_falls_through_to_a_selector_that_matches_nothing_installed() {
    let targets = uninstall_targets(&[], &uninstall_args(Some("edge-01"), false)).unwrap();

    assert_eq!(targets, vec!["edge-01".to_string()]);
}
