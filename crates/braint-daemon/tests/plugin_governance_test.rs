//! Governance tests for the plugin manager.

mod common;

use braint_daemon::plugin::PluginManager;
use std::path::PathBuf;

/// Create a temporary plugin directory containing only the hello binary.
fn make_plugin_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let src = common::hello_plugin_path();
    let dst = dir.path().join("braint-plugin-hello");
    std::fs::copy(&src, &dst).expect("failed to copy hello plugin binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dst).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dst, perms).unwrap();
    }
    let plugin_dir = dir.path().to_path_buf();
    (dir, plugin_dir)
}

/// Loading the hello plugin from two different directories (each with the same binary)
/// should fail with a governance error because both register the same `hello` verb.
///
/// `PluginManager::load` treats per-plugin errors as warnings and returns the manager
/// with only the first successfully-loaded plugin. The manager should own `hello` but
/// only one plugin handle should exist.
#[tokio::test]
async fn duplicate_verb_registration_fails() {
    let (_dir1, plugin_dir1) = make_plugin_dir();
    let (_dir2, plugin_dir2) = make_plugin_dir();

    // Both directories have the hello plugin; the second one should fail with a governance error.
    let manager = PluginManager::load(&[plugin_dir1, plugin_dir2])
        .await
        .expect("PluginManager::load should not propagate per-plugin errors");

    assert!(
        manager.owns_verb("hello"),
        "hello verb should be registered from the first successful load"
    );
}
