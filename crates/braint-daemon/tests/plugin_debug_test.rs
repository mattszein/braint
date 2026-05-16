mod common;

use braint_daemon::plugin::PluginManager;
use std::path::PathBuf;

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

#[tokio::test]
async fn debug_plugin_load() {
    eprintln!("test: getting plugin path");
    let plugin_path = common::hello_plugin_path();
    eprintln!("test: plugin path = {}", plugin_path.display());
    assert!(
        plugin_path.exists(),
        "plugin binary must exist at: {}",
        plugin_path.display()
    );

    eprintln!("test: creating isolated plugin dir");
    let (_dir, plugin_dir) = make_plugin_dir();
    eprintln!("test: loading plugin from {}", plugin_dir.display());

    let mgr = PluginManager::load(&[plugin_dir])
        .await
        .expect("load should succeed");
    eprintln!("test: plugin manager loaded");

    assert!(mgr.owns_verb("hello"), "hello verb should be registered");
    eprintln!("test: owns_verb check passed");
}
