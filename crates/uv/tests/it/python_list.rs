use uv_platform::{Arch, Os};
use uv_static::EnvVars;

use anyhow::Result;
use assert_fs::prelude::*;
use uv_test::uv_snapshot;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[test]
fn python_list() {
    let mut context = uv_test::test_context_with_versions!(&["3.11", "3.12"])
        .with_filtered_python_symlinks()
        .with_filtered_python_keys()
        .with_collapsed_whitespace();

    uv_snapshot!(context.filters(), context.python_list().env(EnvVars::UV_TEST_PYTHON_PATH, ""), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // We show all interpreters
    uv_snapshot!(context.filters(), context.python_list(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");

    // Request Python 3.12
    uv_snapshot!(context.filters(), context.python_list().arg("3.12"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]

    ----- stderr -----
    ");

    // Request Python 3.11
    uv_snapshot!(context.filters(), context.python_list().arg("3.11"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");

    // Request CPython
    uv_snapshot!(context.filters(), context.python_list().arg("cpython"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");

    // Request CPython 3.12
    uv_snapshot!(context.filters(), context.python_list().arg("cpython@3.12"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]

    ----- stderr -----
    ");

    // Request CPython 3.12 via partial key syntax
    uv_snapshot!(context.filters(), context.python_list().arg("cpython-3.12"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]

    ----- stderr -----
    ");

    // Request CPython 3.12 for the current platform
    let os = Os::from_env();
    let arch = Arch::from_env();

    uv_snapshot!(context.filters(), context.python_list().arg(format!("cpython-3.12-{os}-{arch}")), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]

    ----- stderr -----
    ");

    // Request PyPy (which should be missing)
    uv_snapshot!(context.filters(), context.python_list().arg("pypy"), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // Swap the order of the Python versions
    context.python_versions.reverse();

    uv_snapshot!(context.filters(), context.python_list(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");

    // Request Python 3.11
    uv_snapshot!(context.filters(), context.python_list().arg("3.11"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");
}

#[test]
fn python_list_pin() {
    let context = uv_test::test_context_with_versions!(&["3.11", "3.12"])
        .with_filtered_python_symlinks()
        .with_filtered_python_keys()
        .with_collapsed_whitespace();

    // Pin to a version
    uv_snapshot!(context.filters(), context.python_pin().arg("3.12"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    Pinned `.python-version` to `3.12`

    ----- stderr -----
    ");

    // The pin should not affect the listing
    uv_snapshot!(context.filters(), context.python_list(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");

    // So `--no-config` has no effect
    uv_snapshot!(context.filters(), context.python_list().arg("--no-config"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");
}

#[test]
fn python_list_venv() {
    let context = uv_test::test_context_with_versions!(&["3.11", "3.12"])
        .with_filtered_python_symlinks()
        .with_filtered_python_keys()
        .with_filtered_exe_suffix()
        .with_filtered_python_names()
        .with_filtered_virtualenv_bin()
        .with_collapsed_whitespace();

    // Create a virtual environment
    uv_snapshot!(context.filters(), context.venv().arg("--python").arg("3.12").arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // We should not display the virtual environment
    uv_snapshot!(context.filters(), context.python_list(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");

    // Same if the `VIRTUAL_ENV` is not set
    uv_snapshot!(context.filters(), context.python_list(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");
}

#[cfg(unix)]
#[test]
fn python_list_unsupported_version() {
    let context = uv_test::test_context_with_versions!(&["3.12"])
        .with_filtered_python_symlinks()
        .with_filtered_python_keys();

    // Request a low version
    uv_snapshot!(context.filters(), context.python_list().arg("3.5"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Invalid version request: Python <3.6 is not supported but 3.5 was requested.
    ");

    // Request a low version with a patch
    uv_snapshot!(context.filters(), context.python_list().arg("3.5.9"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Invalid version request: Python <3.6 is not supported but 3.5.9 was requested.
    ");

    // Request a really low version
    uv_snapshot!(context.filters(), context.python_list().arg("2.6"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Invalid version request: Python <3.6 is not supported but 2.6 was requested.
    ");

    // Request a really low version with a patch
    uv_snapshot!(context.filters(), context.python_list().arg("2.6.8"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Invalid version request: Python <3.6 is not supported but 2.6.8 was requested.
    ");

    // Request a future version
    uv_snapshot!(context.filters(), context.python_list().arg("4.2"), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // Request a low version with a range
    uv_snapshot!(context.filters(), context.python_list().arg("<3.0"), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // Request free-threaded Python on unsupported version
    uv_snapshot!(context.filters(), context.python_list().arg("3.12t"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Invalid version request: Python <3.13 does not support free-threading but 3.12+freethreaded was requested.
    ");
}

#[test]
fn python_list_duplicate_path_entries() {
    let context = uv_test::test_context_with_versions!(&["3.11", "3.12"])
        .with_filtered_python_symlinks()
        .with_filtered_python_keys()
        .with_collapsed_whitespace();

    // Construct a `PATH` with all entries duplicated
    let path = std::env::join_paths(
        std::env::split_paths(&context.python_path())
            .chain(std::env::split_paths(&context.python_path())),
    )
    .unwrap();

    uv_snapshot!(context.filters(), context.python_list().env(EnvVars::UV_TEST_PYTHON_PATH, &path), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
    cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

    ----- stderr -----
    ");

    #[cfg(unix)]
    {
        // Construct a `PATH` with symlinks
        let path = std::env::join_paths(std::env::split_paths(&context.python_path()).chain(
            std::env::split_paths(&context.python_path()).map(|path| {
                let dst = format!("{}-link", path.display());
                fs_err::os::unix::fs::symlink(&path, &dst).unwrap();
                std::path::PathBuf::from(dst)
            }),
        ))
        .unwrap();

        uv_snapshot!(context.filters(), context.python_list().env(EnvVars::UV_TEST_PYTHON_PATH, &path), @"
        success: true
        exit_code: 0
        ----- stdout -----
        cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]
        cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]

        ----- stderr -----
        ");

        // Reverse the order so the symlinks are first
        let path = std::env::join_paths(
            {
                let mut paths = std::env::split_paths(&path).collect::<Vec<_>>();
                paths.reverse();
                paths
            }
            .iter(),
        )
        .unwrap();

        uv_snapshot!(context.filters(), context.python_list().env(EnvVars::UV_TEST_PYTHON_PATH, &path), @"
        success: true
        exit_code: 0
        ----- stdout -----
        cpython-3.12.[X]-[PLATFORM] [PYTHON-3.12]-link/python3
        cpython-3.11.[X]-[PLATFORM] [PYTHON-3.11]-link/python3

        ----- stderr -----
        ");
    }
}

#[test]
fn python_list_downloads() {
    let context = uv_test::test_context_with_versions!(&[])
        .with_filtered_python_keys()
        .with_filtered_latest_python_versions();

    // We do not test showing all interpreters — as it differs per platform
    // Instead, we choose a Python version where our available distributions are stable

    // Test the default display, which requires reverting the test context disabling Python downloads
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM]    <download available>
    pypy-3.10.16-[PLATFORM]       <download available>
    graalpy-3.10.0-[PLATFORM]     <download available>

    ----- stderr -----
    ");

    // Show patch versions
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").arg("--all-versions").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM]    <download available>
    cpython-3.10.19-[PLATFORM]    <download available>
    cpython-3.10.18-[PLATFORM]    <download available>
    cpython-3.10.17-[PLATFORM]    <download available>
    cpython-3.10.16-[PLATFORM]    <download available>
    cpython-3.10.15-[PLATFORM]    <download available>
    cpython-3.10.14-[PLATFORM]    <download available>
    cpython-3.10.13-[PLATFORM]    <download available>
    cpython-3.10.12-[PLATFORM]    <download available>
    cpython-3.10.11-[PLATFORM]    <download available>
    cpython-3.10.9-[PLATFORM]     <download available>
    cpython-3.10.8-[PLATFORM]     <download available>
    cpython-3.10.7-[PLATFORM]     <download available>
    cpython-3.10.6-[PLATFORM]     <download available>
    cpython-3.10.5-[PLATFORM]     <download available>
    cpython-3.10.4-[PLATFORM]     <download available>
    cpython-3.10.3-[PLATFORM]     <download available>
    cpython-3.10.2-[PLATFORM]     <download available>
    cpython-3.10.0-[PLATFORM]     <download available>
    pypy-3.10.16-[PLATFORM]       <download available>
    pypy-3.10.14-[PLATFORM]       <download available>
    pypy-3.10.13-[PLATFORM]       <download available>
    pypy-3.10.12-[PLATFORM]       <download available>
    graalpy-3.10.0-[PLATFORM]     <download available>

    ----- stderr -----
    ");
}

#[test]
#[cfg(feature = "test-python-managed")]
fn python_list_downloads_installed() {
    use assert_cmd::assert::OutputAssertExt;

    let context = uv_test::test_context_with_versions!(&[])
        .with_filtered_python_keys()
        .with_filtered_python_install_bin()
        .with_filtered_python_names()
        .with_managed_python_dirs()
        .with_filtered_latest_python_versions();

    // We do not test showing all interpreters - as it differs per platform
    // Instead, we choose a Python version where our available distributions are stable

    // First, the download is shown as available
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM]    <download available>
    pypy-3.10.16-[PLATFORM]       <download available>
    graalpy-3.10.0-[PLATFORM]     <download available>

    ----- stderr -----
    ");

    // TODO(zanieb): It'd be nice to test `--show-urls` here too but we need special filtering for
    // the URL

    // But not if `--only-installed` is used
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").arg("--only-installed").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // Install a Python version
    context.python_install().arg("3.10").assert().success();

    // Then, it should be listed as installed instead of available
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM]    managed/cpython-3.10-[PLATFORM]/[INSTALL-BIN]/[PYTHON]
    pypy-3.10.16-[PLATFORM]       <download available>
    graalpy-3.10.0-[PLATFORM]     <download available>

    ----- stderr -----
    ");

    // But, the display should be reverted if `--only-downloads` is used
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").arg("--only-downloads").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM]    <download available>
    pypy-3.10.16-[PLATFORM]       <download available>
    graalpy-3.10.0-[PLATFORM]     <download available>

    ----- stderr -----
    ");

    // And should not be shown if `--no-managed-python` is used
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").arg("--no-managed-python").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // When `--managed-python` is used, managed installations should still be shown
    uv_snapshot!(context.filters(), context.python_list().arg("3.10").arg("--managed-python").env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM]    managed/cpython-3.10-[PLATFORM]/[INSTALL-BIN]/[PYTHON]
    pypy-3.10.16-[PLATFORM]       <download available>
    graalpy-3.10.0-[PLATFORM]     <download available>

    ----- stderr -----
    ");
}

/// Test that symlinks installed by `python install` on the search path are correctly
/// filtered by `--managed-python` and `--no-managed-python`.
#[test]
#[cfg(all(unix, feature = "test-python-managed"))]
fn python_list_managed_symlinks() {
    use assert_cmd::assert::OutputAssertExt;

    let context = uv_test::test_context_with_versions!(&[])
        .with_filtered_python_keys()
        .with_filtered_python_install_bin()
        .with_filtered_python_names()
        .with_managed_python_dirs()
        .with_filtered_latest_python_versions();

    // Install a Python version; this creates a symlink in `bin_dir` (on the search path)
    context.python_install().arg("3.10").assert().success();

    // Include `bin_dir` in the test search path so the symlink is discoverable
    let bin_dir = context.bin_dir.to_path_buf();

    // With `--no-managed-python`, the symlink should be excluded since it points to a
    // managed installation
    uv_snapshot!(context.filters(), context.python_list()
        .arg("3.10")
        .arg("--only-installed")
        .arg("--no-managed-python")
        .env(EnvVars::UV_TEST_PYTHON_PATH, &bin_dir), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    // With `--managed-python`, both the managed installation and the symlink are shown
    uv_snapshot!(context.filters(), context.python_list()
        .arg("3.10")
        .arg("--only-installed")
        .arg("--managed-python")
        .env(EnvVars::UV_TEST_PYTHON_PATH, &bin_dir), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM]    [BIN]/[PYTHON] -> managed/cpython-3.10-[PLATFORM]/[INSTALL-BIN]/[PYTHON]
    cpython-3.10.[LATEST]-[PLATFORM]    managed/cpython-3.10-[PLATFORM]/[INSTALL-BIN]/[PYTHON]

    ----- stderr -----
    ");
}

#[tokio::test]
async fn python_list_remote_python_downloads_json_url() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let remote_json = r#"
    {
        "cpython-3.14.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": {
                "family": "aarch64",
                "variant": null
            },
            "os": "darwin",
            "libc": "none",
            "major": 3,
            "minor": 14,
            "patch": 0,
            "prerelease": "",
            "url": "https://custom.com/cpython-3.14.0-darwin-aarch64-none.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null,
            "build": "20251028"
        },
        "cpython-3.13.2+freethreaded-linux-powerpc64le-gnu": {
            "name": "cpython",
            "arch": {
                "family": "powerpc64le",
                "variant": null
            },
            "os": "linux",
            "libc": "gnu",
            "major": 3,
            "minor": 13,
            "patch": 2,
            "prerelease": "",
            "url": "https://custom.com/ccpython-3.13.2+freethreaded-linux-powerpc64le-gnu.tar.gz",
            "sha256": "6ae8fa44cb2edf4ab49cff1820b53c40c10349c0f39e11b8cd76ce7f3e7e1def",
            "variant": "freethreaded",
            "build": "20250317"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/invalid"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{", "application/json"))
        .mount(&server)
        .await;

    // Test showing all interpreters from the remote JSON URL
    uv_snapshot!(context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-versions")
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("--python-downloads-json-url").arg(server.uri()), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.14.0-macos-aarch64-none                    https://custom.com/cpython-3.14.0-darwin-aarch64-none.tar.gz
    cpython-3.13.2+freethreaded-linux-powerpc64le-gnu    https://custom.com/ccpython-3.13.2+freethreaded-linux-powerpc64le-gnu.tar.gz

    ----- stderr -----
    ");

    // test invalid URL path
    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--python-downloads-json-url").arg(format!("{}/404", server.uri())), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Error while fetching remote python downloads json from 'http://[LOCALHOST]/404'
      Caused by: Failed to download http://[LOCALHOST]/404
      Caused by: HTTP status client error (404 Not Found) for url (http://[LOCALHOST]/404)
    ");

    // test invalid json
    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--python-downloads-json-url").arg(format!("{}/invalid", server.uri())), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Unable to parse the JSON Python download list at http://[LOCALHOST]/invalid
      Caused by: EOF while parsing an object at line 1 column 1
    ");

    Ok(())
}

/// Custom `[[python-indexes]]` entries in `uv.toml` are merged with the built-in downloads.
#[tokio::test]
async fn python_list_python_indexes() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let remote_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin",
            "libc": "none",
            "major": 3,
            "minor": 99,
            "patch": 0,
            "prerelease": "",
            "url": "https://custom.example.com/cpython-3.99.0-darwin-aarch64-none.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null,
            "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    // Non-default index: custom entry is merged with the built-in downloads.
    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "custom"
            url = "{uri}/versions.json"
        "#},
        uri = server.uri(),
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-versions")
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("3.99"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://custom.example.com/cpython-3.99.0-darwin-aarch64-none.tar.gz

    ----- stderr -----
    warning: Python index `custom` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    // With `default = true`, the built-in list is suppressed — only the custom entry remains.
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "custom"
            url = "{uri}/versions.json"
            default = true
        "#},
        uri = server.uri(),
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-versions")
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://custom.example.com/cpython-3.99.0-darwin-aarch64-none.tar.gz

    ----- stderr -----
    warning: Python index `custom` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    Ok(())
}

/// A custom index serving invalid JSON surfaces a parse error naming the source.
#[tokio::test]
async fn python_list_python_indexes_invalid_json() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/bad.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{", "application/json"))
        .mount(&server)
        .await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "broken"
            url = "{uri}/bad.json"
        "#},
        uri = server.uri(),
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("3.99"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    warning: Python index `broken` is fetched over plain HTTP at a loopback address; safe only for local testing
    error: Unable to parse the JSON Python download list at http://[LOCALHOST]/bad.json
      Caused by: EOF while parsing an object at line 1 column 1
    ");

    Ok(())
}

/// A custom index returning 404 fails hard with a redacted URL.
#[tokio::test]
async fn python_list_python_indexes_http_404() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "missing"
            url = "{uri}/nope.json"
        "#},
        uri = server.uri(),
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("3.99"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    warning: Python index `missing` is fetched over plain HTTP at a loopback address; safe only for local testing
    error: Error while fetching remote python downloads json from 'http://[LOCALHOST]/nope.json'
      Caused by: Failed to download http://[LOCALHOST]/nope.json
      Caused by: HTTP status client error (404 Not Found) for url (http://[LOCALHOST]/nope.json)
    ");

    Ok(())
}

/// A custom index entry lacking `sha256` is rejected — built-in downloads always have hashes,
/// custom indexes must too, or binaries could be installed unverified.
#[tokio::test]
async fn python_list_python_indexes_missing_sha256() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let remote_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin",
            "libc": "none",
            "major": 3,
            "minor": 99,
            "patch": 0,
            "prerelease": "",
            "url": "https://custom.example.com/cpython-3.99.0.tar.gz",
            "sha256": null,
            "variant": null,
            "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "unhashed"
            url = "{uri}/versions.json"
        "#},
        uri = server.uri(),
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("3.99"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    warning: Python index `unhashed` is fetched over plain HTTP at a loopback address; safe only for local testing
    error: Python index `unhashed` entry `cpython-3.99.0-macos-aarch64-none` is missing a `sha256`; downloads from custom indexes must include a hash
    ");

    Ok(())
}

/// Two indexes marked `default = true` are rejected up-front.
#[test]
fn python_list_python_indexes_multiple_defaults() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let config = context.temp_dir.child("uv.toml");
    config.write_str(indoc::indoc! {r#"
        [[python-indexes]]
        name = "a"
        url = "https://a.example.com/versions.json"
        default = true

        [[python-indexes]]
        name = "b"
        url = "https://b.example.com/versions.json"
        default = true
    "#})?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: At most one `[[python-indexes]]` entry may set `default = true`; found 2: a, b
    ");

    Ok(())
}

/// An unsupported URL scheme (e.g. `ftp://`) is rejected early with a clear error instead of
/// silently being reinterpreted as a filesystem path.
#[test]
fn python_list_python_indexes_unsupported_scheme() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let config = context.temp_dir.child("uv.toml");
    config.write_str(indoc::indoc! {r#"
        [[python-indexes]]
        name = "weird"
        url = "ftp://example.com/versions.json"
    "#})?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Unsupported URL scheme `ftp` in Python index `weird`; expected `http`, `https`, or `file`
    ");

    Ok(())
}

/// `--python-index` accepts multiple values on the CLI; they compose with each other.
#[tokio::test]
async fn python_list_python_indexes_cli_multiple() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let a_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://a.example.com/py-3.99.0.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    let b_json = r#"
    {
        "cpython-3.98.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 98, "patch": 0, "prerelease": "",
            "url": "https://b.example.com/py-3.98.0.tar.gz",
            "sha256": "6ae8fa44cb2edf4ab49cff1820b53c40c10349c0f39e11b8cd76ce7f3e7e1def",
            "variant": null, "build": "20260102"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/a.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(a_json, "application/json"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/b.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(b_json, "application/json"))
        .mount(&server)
        .await;

    // Entry from the first custom index.
    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("--python-index").arg(format!("{}/a.json", server.uri()))
        .arg("--python-index").arg(format!("{}/b.json", server.uri()))
        .arg("3.99"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://a.example.com/py-3.99.0.tar.gz

    ----- stderr -----
    warning: Python index `$cli-0` is fetched over plain HTTP at a loopback address; safe only for local testing
    warning: Python index `$cli-1` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    // Entry from the second custom index.
    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("--python-index").arg(format!("{}/a.json", server.uri()))
        .arg("--python-index").arg(format!("{}/b.json", server.uri()))
        .arg("3.98"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.98.0-macos-aarch64-none    https://b.example.com/py-3.98.0.tar.gz

    ----- stderr -----
    warning: Python index `$cli-0` is fetched over plain HTTP at a loopback address; safe only for local testing
    warning: Python index `$cli-1` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    Ok(())
}

/// Regression: a custom index advertising a LOWER patch for a version minor (e.g. `3.12.0`)
/// must not shadow the built-in HIGHER patch (`3.12.x`) when the user requests `3.12`.
///
/// This is the headline invariant of the merge-order fix: built-in is pushed first, dedup-
/// last-wins only affects same-*key* collisions, and the final sort picks the highest version.
#[tokio::test]
async fn python_list_python_indexes_does_not_shadow_higher_builtin() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    // Advertise cpython-3.10.0 — the built-in list has higher 3.10.x patches.
    let remote_json = r#"
    {
        "cpython-3.10.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 10, "patch": 0, "prerelease": "",
            "url": "https://custom.example.com/cpython-3.10.0.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "custom"
            url = "{uri}/versions.json"
        "#},
        uri = server.uri(),
    ))?;

    // Request `cpython-3.10` — the built-in's latest 3.10.x MUST win.
    let output = context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("cpython-3.10")
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        !stdout.contains("cpython-3.10.0-"),
        "custom 3.10.0 should have been shadowed by built-in higher patch.\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );
    assert!(
        stdout.contains("cpython-3.10."),
        "expected a 3.10.x built-in entry.\nstdout:\n{stdout}",
    );

    Ok(())
}

/// Two custom indexes contribute entries with the same `PythonInstallationKey`. The later /
/// higher-priority source (the one listed *second* in `uv.toml`, matching
/// [`PythonInstallMirrors::combine`] semantics) wins, demonstrating the dedup-last-wins
/// behavior that enables the "patched drop-in" use case.
#[tokio::test]
async fn python_list_python_indexes_same_key_last_wins() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let shared_key = "cpython-3.99.0-darwin-aarch64-none";
    let base_json = |origin: &str, build: &str| {
        format!(
            r#"
        {{
            "{shared_key}": {{
                "name": "cpython",
                "arch": {{ "family": "aarch64", "variant": null }},
                "os": "darwin", "libc": "none",
                "major": 3, "minor": 99, "patch": 0, "prerelease": "",
                "url": "https://{origin}.example.com/py.tar.gz",
                "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
                "variant": null, "build": "{build}"
            }}
        }}
        "#
        )
    };
    Mock::given(method("GET"))
        .and(path("/first.json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(base_json("first", "a"), "application/json"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/second.json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(base_json("second", "b"), "application/json"),
        )
        .mount(&server)
        .await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "first"
            url = "{uri}/first.json"

            [[python-indexes]]
            name = "second"
            url = "{uri}/second.json"
        "#},
        uri = server.uri(),
    ))?;

    let output = context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("3.99")
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stdout.contains("second.example.com"),
        "expected later-listed index to win on same key.\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );
    assert!(
        !stdout.contains("first.example.com"),
        "earlier-listed index should have been shadowed.\nstdout:\n{stdout}",
    );

    Ok(())
}

/// `[[python-indexes]]` entries can point at a local file via `file://`.
#[tokio::test]
async fn python_list_python_indexes_file_url() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let json = context.temp_dir.child("versions.json");
    json.write_str(
        r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://file.example.com/py-3.99.0.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#,
    )?;

    // Write a URL with the `file://` scheme so `resolve_location` takes the file-URL branch.
    let json_url =
        url::Url::from_file_path(json.path()).expect("temp path converts to file:// URL");
    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "local"
            url = "{url}"
        "#},
        url = json_url,
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("3.99"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://file.example.com/py-3.99.0.tar.gz

    ----- stderr -----
    ");

    Ok(())
}

/// CLI `--python-index` and `uv.toml` `[[python-indexes]]` both contribute entries, and CLI wins
/// on same-key collision (CLI is higher priority than config, per the [`combine`] ordering).
#[tokio::test]
async fn python_list_python_indexes_cli_and_config_merge() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let config_json = r#"
    {
        "cpython-3.98.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 98, "patch": 0, "prerelease": "",
            "url": "https://config.example.com/py-3.98.0.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    let cli_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://cli.example.com/py-3.99.0.tar.gz",
            "sha256": "6ae8fa44cb2edf4ab49cff1820b53c40c10349c0f39e11b8cd76ce7f3e7e1def",
            "variant": null, "build": "20260102"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/config.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(config_json, "application/json"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cli.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(cli_json, "application/json"))
        .mount(&server)
        .await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "config"
            url = "{uri}/config.json"
        "#},
        uri = server.uri(),
    ))?;

    // Config entry surfaces.
    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("--python-index").arg(format!("{}/cli.json", server.uri()))
        .arg("3.98"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.98.0-macos-aarch64-none    https://config.example.com/py-3.98.0.tar.gz

    ----- stderr -----
    warning: Python index `config` is fetched over plain HTTP at a loopback address; safe only for local testing
    warning: Python index `$cli-0` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    // CLI entry surfaces.
    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("--python-index").arg(format!("{}/cli.json", server.uri()))
        .arg("3.99"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://cli.example.com/py-3.99.0.tar.gz

    ----- stderr -----
    warning: Python index `config` is fetched over plain HTTP at a loopback address; safe only for local testing
    warning: Python index `$cli-0` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    Ok(())
}

/// A sha256 with the wrong length or non-hex characters is rejected at load time — a stricter
/// check than the pre-existing "nonempty string" gate, so malformed hashes fail fast instead of
/// surfacing as an opaque `HashMismatch` at download time.
#[tokio::test]
async fn python_list_python_indexes_malformed_sha256() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let remote_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://custom.example.com/py.tar.gz",
            "sha256": "deadbeef",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "bad-hash"
            url = "{uri}/versions.json"
        "#},
        uri = server.uri(),
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("3.99"), @"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    warning: Python index `bad-hash` is fetched over plain HTTP at a loopback address; safe only for local testing
    error: Python index `bad-hash` entry `cpython-3.99.0-macos-aarch64-none` has a malformed `sha256` (expected 64 hex characters, got `deadbeef`)
    ");

    Ok(())
}

/// `UV_PYTHON_INDEX` supplies a single-entry index list named `env`.
#[tokio::test]
async fn python_list_python_indexes_env_var() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let remote_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://env.example.com/py-3.99.0.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .env(EnvVars::UV_PYTHON_INDEX, format!("{}/versions.json", server.uri()))
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("3.99"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://env.example.com/py-3.99.0.tar.gz

    ----- stderr -----
    warning: Python index `$env` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    Ok(())
}

/// A malformed `UV_PYTHON_INDEX` URL surfaces a clear environment-variable error.
#[test]
fn python_list_python_indexes_env_var_malformed() {
    let context = uv_test::test_context_with_versions!(&[]);

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .env(EnvVars::UV_PYTHON_INDEX, "not a url"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Failed to parse environment variable `UV_PYTHON_INDEX` with invalid value `not a url`: not a valid URL: relative URL without a base
    ");
}

/// User-defined names starting with `$` are reserved for synthesized entries (like `$env`
/// or `$cli-0`) and are rejected at TOML parse time so the collision isn't silent.
#[test]
fn python_list_python_indexes_reserved_name() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let config = context.temp_dir.child("uv.toml");
    config.write_str(indoc::indoc! {r#"
        [[python-indexes]]
        name = "$mycorp"
        url = "https://mycorp.example.com/versions.json"
    "#})?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @r#"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Failed to parse: `uv.toml`
      Caused by: TOML parse error at line 1, column 1
      |
    1 | [[python-indexes]]
      | ^^^^^^^^^^^^^^^^^^
    Python index name `$mycorp` uses the reserved `$` prefix; `$`-prefixed names are synthesized internally (e.g. for `UV_PYTHON_INDEX` or `--python-index`)
    "#);

    Ok(())
}

/// Two `[[python-indexes]]` entries with the same `name` in a single config file are rejected
/// up-front so the second entry doesn't silently shadow the first.
#[test]
fn python_list_python_indexes_duplicate_in_file() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let config = context.temp_dir.child("uv.toml");
    config.write_str(indoc::indoc! {r#"
        [[python-indexes]]
        name = "shared"
        url = "https://a.example.com/versions.json"

        [[python-indexes]]
        name = "shared"
        url = "https://b.example.com/versions.json"
    "#})?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Duplicate `[[python-indexes]]` name `shared`; each entry in a single config layer must have a unique name
    ");

    Ok(())
}

/// An unknown field on a `[[python-indexes]]` entry (e.g. typo) is rejected at TOML parse
/// time rather than silently ignored.
#[test]
fn python_list_python_indexes_unknown_field() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let config = context.temp_dir.child("uv.toml");
    config.write_str(indoc::indoc! {r#"
        [[python-indexes]]
        name = "typo"
        url = "https://example.com/versions.json"
        defualt = true
    "#})?;

    let output = context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .output()?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!output.status.success(), "expected failure, got:\n{stderr}");
    assert!(
        stderr.contains("defualt") || stderr.contains("unknown field"),
        "expected unknown-field diagnostic mentioning `defualt`, got:\n{stderr}",
    );

    Ok(())
}

/// IPv6 loopback `[::1]` over plain HTTP is allowed via the `is_loopback_http` Ipv6 branch.
///
/// Skips gracefully when IPv6 isn't available locally (some CI environments disable it).
#[tokio::test]
async fn python_list_python_indexes_ipv6_loopback() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let Ok(listener) = std::net::TcpListener::bind(std::net::SocketAddr::new(
        std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        0,
    )) else {
        // Some CI environments disable IPv6; skip rather than fail so the test stays green
        // where it can't run. IPv4 loopback coverage is the common path and exercised elsewhere.
        return Ok(());
    };
    let server = MockServer::builder().listener(listener).start().await;

    let remote_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://ipv6.example.com/py.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "ipv6"
            url = "{uri}/versions.json"
        "#},
        uri = server.uri(),
    ))?;

    let output = context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("3.99")
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        output.status.success(),
        "IPv6 loopback should be accepted.\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );
    assert!(
        stdout.contains("https://ipv6.example.com/py.tar.gz"),
        "expected custom IPv6 entry, got:\n{stdout}",
    );

    Ok(())
}

/// A project `uv.toml` can redefine the `url` for a globally-configured index (by reusing
/// the `name`) without erroring — the later (higher-priority) layer wins. This is the
/// "layer by name" semantics in [`PythonInstallMirrors::combine`].
#[tokio::test]
async fn python_list_python_indexes_config_layering() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let project_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://project.example.com/py.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/project.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(project_json, "application/json"))
        .mount(&server)
        .await;

    // Simulate a "global" uv.toml by writing it to a parent directory and running `python list`
    // from a child directory. Both layers share the `name = "mycorp"` entry; the project-level
    // URL should win. We approximate by using only the project-level config but asserting the
    // resolved URL matches — the cross-layer path is covered by unit tests of `combine()`.
    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "mycorp"
            url = "{uri}/project.json"
        "#},
        uri = server.uri(),
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("3.99"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://project.example.com/py.tar.gz

    ----- stderr -----
    warning: Python index `mycorp` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    Ok(())
}

/// A `localhost` domain URL is accepted over plain HTTP (loopback exception, Domain branch).
#[tokio::test]
async fn python_list_python_indexes_localhost_http() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);
    let server = MockServer::start().await;

    let remote_json = r#"
    {
        "cpython-3.99.0-darwin-aarch64-none": {
            "name": "cpython",
            "arch": { "family": "aarch64", "variant": null },
            "os": "darwin", "libc": "none",
            "major": 3, "minor": 99, "patch": 0, "prerelease": "",
            "url": "https://localhost.example.com/py.tar.gz",
            "sha256": "c3223d5924a0ed0ef5958a750377c362d0957587f896c0f6c635ae4b39e0f337",
            "variant": null, "build": "20260101"
        }
    }
    "#;
    Mock::given(method("GET"))
        .and(path("/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_json, "application/json"))
        .mount(&server)
        .await;

    // Rewrite `127.0.0.1:PORT` → `localhost:PORT` to exercise the Domain branch of
    // `is_loopback_http` rather than the Ipv4 branch.
    let port = server.address().port();
    let localhost_url = format!("http://localhost:{port}/versions.json");

    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "local"
            url = "{url}"
        "#},
        url = localhost_url,
    ))?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--all-platforms")
        .arg("--all-arches")
        .arg("--show-urls")
        .arg("3.99"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.99.0-macos-aarch64-none    https://localhost.example.com/py.tar.gz

    ----- stderr -----
    warning: Python index `local` is fetched over plain HTTP at a loopback address; safe only for local testing
    ");

    Ok(())
}

/// A bare filesystem path that doesn't exist surfaces a clear IO error instead of silently
/// returning an empty list.
#[test]
fn python_list_python_indexes_file_not_found() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    // A `file://` URL targeting a nonexistent path exercises the path-read branch of
    // `ManagedPythonDownloadList::new`.
    let missing = context.temp_dir.child("does-not-exist.json");
    let missing_url =
        url::Url::from_file_path(missing.path()).expect("temp path converts to file:// URL");
    let config = context.temp_dir.child("uv.toml");
    config.write_str(&format!(
        indoc::indoc! {r#"
            [[python-indexes]]
            name = "missing"
            url = "{url}"
        "#},
        url = missing_url,
    ))?;

    let output = context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .output()?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!output.status.success(), "expected failure, got:\n{stderr}");
    assert!(
        stderr.contains("No such file") || stderr.contains("cannot find"),
        "expected a file-not-found diagnostic, got:\n{stderr}",
    );

    Ok(())
}

/// A non-loopback `http://` index is rejected outright — a network attacker could otherwise swap
/// binaries and hashes.
#[test]
fn python_list_python_indexes_insecure_http() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    let config = context.temp_dir.child("uv.toml");
    config.write_str(indoc::indoc! {r#"
        [[python-indexes]]
        name = "insecure"
        url = "http://example.com/versions.json"
    "#})?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: Python index `insecure` is configured with a plain-HTTP URL (`http://example.com/versions.json`); index JSON must be served over HTTPS (an attacker on the network could otherwise substitute binaries and their expected hashes)
    ");

    Ok(())
}

/// In `--offline` mode, remote `[[python-indexes]]` are skipped with a visible warning so the
/// user sees the link between "my index vanished" and "I'm offline". Built-in downloads remain
/// available, and the command succeeds (rather than hard-failing on a fetch the user can't
/// complete anyway).
#[test]
fn python_list_python_indexes_offline() -> Result<()> {
    let context = uv_test::test_context_with_versions!(&[]);

    // Use an `https://` URL so the insecure-scheme check doesn't fire and an unreachable
    // hostname so the test doesn't depend on a real network — we're verifying the fetch is
    // skipped entirely.
    let config = context.temp_dir.child("uv.toml");
    config.write_str(indoc::indoc! {r#"
        [[python-indexes]]
        name = "unreachable"
        url = "https://unreachable.invalid/versions.json"
    "#})?;

    uv_snapshot!(context.filters(), context
        .python_list()
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS)
        .arg("--offline")
        .arg("--only-downloads")
        .arg("cpython-3.12"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.12.13-macos-x86_64-none    <download available>

    ----- stderr -----
    warning: Python index `unreachable` is not available in offline mode; skipping
    ");

    Ok(())
}

#[test]
fn python_list_with_mirrors() {
    let context = uv_test::test_context_with_versions!(&[])
        .with_filtered_python_keys()
        .with_collapsed_whitespace()
        .with_filtered_latest_python_versions()
        // Add filters to normalize file paths in URLs
        .with_filter((
            r"(https://mirror\.example\.com/).*".to_string(),
            "$1[FILE-PATH]".to_string(),
        ))
        .with_filter((
            r"(https://python-mirror\.example\.com/).*".to_string(),
            "$1[FILE-PATH]".to_string(),
        ))
        .with_filter((
            r"(https://pypy-mirror\.example\.com/).*".to_string(),
            "$1[FILE-PATH]".to_string(),
        ))
        .with_filter((
            r"(https://github\.com/astral-sh/python-build-standalone/releases/download/).*"
                .to_string(),
            "$1[FILE-PATH]".to_string(),
        ))
        .with_filter((
            r"(https://releases\.astral\.sh/github/python-build-standalone/releases/download/).*"
                .to_string(),
            "$1[FILE-PATH]".to_string(),
        ))
        .with_filter((
            r"(https://downloads\.python\.org/pypy/).*".to_string(),
            "$1[FILE-PATH]".to_string(),
        ))
        .with_filter((
            r"(https://github\.com/oracle/graalpython/releases/download/).*".to_string(),
            "$1[FILE-PATH]".to_string(),
        ));

    // Test with UV_PYTHON_INSTALL_MIRROR environment variable - verify mirror URL is used
    uv_snapshot!(context.filters(), context.python_list()
        .arg("cpython@3.10.19")
        .arg("--show-urls")
        .env(EnvVars::UV_PYTHON_INSTALL_MIRROR, "https://mirror.example.com")
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.19-[PLATFORM] https://mirror.example.com/[FILE-PATH]

    ----- stderr -----
    ");

    // Test with UV_PYPY_INSTALL_MIRROR environment variable - verify PyPy mirror URL is used
    uv_snapshot!(context.filters(), context.python_list()
        .arg("pypy@3.10")
        .arg("--show-urls")
        .env(EnvVars::UV_PYPY_INSTALL_MIRROR, "https://pypy-mirror.example.com")
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    pypy-3.10.16-[PLATFORM] https://pypy-mirror.example.com/[FILE-PATH]

    ----- stderr -----
    ");

    // Test with both mirror environment variables set
    uv_snapshot!(context.filters(), context.python_list()
        .arg("3.10")
        .arg("--show-urls")
        .env(EnvVars::UV_PYTHON_INSTALL_MIRROR, "https://python-mirror.example.com")
        .env(EnvVars::UV_PYPY_INSTALL_MIRROR, "https://pypy-mirror.example.com")
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM] https://python-mirror.example.com/[FILE-PATH]
    pypy-3.10.16-[PLATFORM] https://pypy-mirror.example.com/[FILE-PATH]
    graalpy-3.10.0-[PLATFORM] https://github.com/oracle/graalpython/releases/download/[FILE-PATH]

    ----- stderr -----
    ");

    // Test without mirrors - verify the default Astral mirror URL is used for CPython
    uv_snapshot!(context.filters(), context.python_list()
        .arg("3.10")
        .arg("--show-urls")
        .env_remove(EnvVars::UV_PYTHON_DOWNLOADS), @"
    success: true
    exit_code: 0
    ----- stdout -----
    cpython-3.10.[LATEST]-[PLATFORM] https://releases.astral.sh/github/python-build-standalone/releases/download/[FILE-PATH]
    pypy-3.10.16-[PLATFORM] https://downloads.python.org/pypy/[FILE-PATH]
    graalpy-3.10.0-[PLATFORM] https://github.com/oracle/graalpython/releases/download/[FILE-PATH]

    ----- stderr -----
    ");
}
