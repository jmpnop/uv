use anyhow::Result;
use std::fmt::Write;
use std::path::Path;

use uv_cache::Cache;
use uv_client::BaseClientBuilder;
use uv_configuration::{DependencyGroupsWithDefaults, PythonIndex};
use uv_errors::ErrorWithHints;
use uv_fs::Simplified;
use uv_python::downloads::ManagedPythonDownloadList;
use uv_python::{
    EnvironmentPreference, PythonDownloads, PythonInstallation, PythonPreference, PythonRequest,
};
use uv_scripts::Pep723ItemRef;
use uv_settings::PythonInstallMirrors;
use uv_warnings::{warn_user, warn_user_once};
use uv_workspace::{DiscoveryOptions, VirtualProject, WorkspaceCache, WorkspaceErrorKind};

use crate::commands::{
    ExitStatus,
    project::{ScriptInterpreter, WorkspacePython, validate_project_requires_python},
};
use crate::printer::Printer;

/// Find a Python interpreter.
#[expect(clippy::fn_params_excessive_bools)]
pub(crate) async fn find(
    project_dir: &Path,
    request: Option<String>,
    show_version: bool,
    resolve_links: bool,
    no_project: bool,
    no_config: bool,
    system: bool,
    python_preference: PythonPreference,
    python_downloads_json_url: Option<&str>,
    python_indexes: Option<&[PythonIndex]>,
    client_builder: &BaseClientBuilder<'_>,
    cache: &Cache,
    workspace_cache: &WorkspaceCache,
    printer: Printer,
) -> Result<ExitStatus> {
    let environment_preference = if system {
        EnvironmentPreference::OnlySystem
    } else {
        EnvironmentPreference::Any
    };

    let project = if no_project {
        None
    } else {
        match VirtualProject::discover(
            project_dir,
            &DiscoveryOptions::default(),
            cache,
            workspace_cache,
        )
        .await
        {
            Ok(project) => Some(project),
            Err(err) => {
                // Ignore missing or unmanaged workspaces in Python discovery.
                if !matches!(
                    err.as_ref(),
                    WorkspaceErrorKind::MissingProject(_)
                        | WorkspaceErrorKind::MissingPyprojectToml
                        | WorkspaceErrorKind::NonWorkspace(_)
                ) {
                    warn_user_once!("{err}");
                }
                None
            }
        }
    };

    // Don't enable the requires-python settings on groups
    let groups = DependencyGroupsWithDefaults::none();
    let WorkspacePython {
        source,
        python_request,
        requires_python,
    } = WorkspacePython::from_request(
        request.map(|request| PythonRequest::parse(&request)),
        project.as_ref().map(VirtualProject::workspace),
        &groups,
        project_dir,
        no_config,
    )
    .await?;

    let client = client_builder.clone().retries(0).build()?;
    let download_list =
        ManagedPythonDownloadList::new(&client, python_downloads_json_url, python_indexes).await?;

    // `find` warns about an outdated prerelease using the download list, which honors any custom
    // `[[python-indexes]]`, so no separate `download_and_warn_if_outdated_prerelease` fetch is needed.
    let python = PythonInstallation::find(
        &python_request.unwrap_or_default(),
        environment_preference,
        python_preference,
        &download_list,
        cache,
    )?;

    // Warn if the discovered Python version is incompatible with the current workspace
    if let Some(requires_python) = requires_python {
        match validate_project_requires_python(
            python.interpreter(),
            project.as_ref().map(VirtualProject::workspace),
            &groups,
            &requires_python,
            &source,
        ) {
            Ok(()) => {}
            Err(err) => {
                warn_user!("{err}");
            }
        }
    }

    if show_version {
        writeln!(
            printer.stdout(),
            "{}",
            python.interpreter().python_version()
        )?;
    } else {
        let path = if resolve_links {
            dunce::canonicalize(python.interpreter().sys_executable())?
        } else {
            std::path::absolute(python.interpreter().sys_executable())?
        };
        writeln!(printer.stdout(), "{}", path.simplified_display())?;
    }

    Ok(ExitStatus::Success)
}

pub(crate) async fn find_script(
    script: Pep723ItemRef<'_>,
    show_version: bool,
    resolve_links: bool,
    client_builder: &BaseClientBuilder<'_>,
    python_preference: PythonPreference,
    python_downloads: PythonDownloads,
    no_config: bool,
    cache: &Cache,
    printer: Printer,
) -> Result<ExitStatus> {
    let interpreter = match ScriptInterpreter::discover(
        script,
        None,
        client_builder,
        python_preference,
        python_downloads,
        &PythonInstallMirrors::default(),
        false,
        no_config,
        Some(false),
        cache,
        printer,
    )
    .await
    {
        Err(error) => {
            writeln!(
                printer.stderr(),
                "{}",
                ErrorWithHints::new(&error, uv_errors::Hint::hints(&error))
            )?;
            return Ok(ExitStatus::Failure);
        }
        Ok(ScriptInterpreter::Interpreter(interpreter)) => interpreter,
        Ok(ScriptInterpreter::Environment(environment)) => environment.into_interpreter(),
    };

    if show_version {
        writeln!(printer.stdout(), "{}", interpreter.python_version())?;
    } else {
        let path = if resolve_links {
            dunce::canonicalize(interpreter.sys_executable())?
        } else {
            std::path::absolute(interpreter.sys_executable())?
        };
        writeln!(printer.stdout(), "{}", path.simplified_display())?;
    }

    Ok(ExitStatus::Success)
}
