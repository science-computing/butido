use anyhow::anyhow;
use anyhow::Result;
use clap::ArgMatches;
use log::{error, info, trace};
use tokio::stream::StreamExt;

use crate::config::*;
use crate::package::Shebang;
use crate::repository::Repository;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::ScriptBuilder;
use crate::util::progress::ProgressBars;

pub async fn lint(matches: &ArgMatches, progressbars: ProgressBars, config: &Configuration, repo: Repository) -> Result<()> {
    let linter = config.script_linter()
        .as_ref()
        .ok_or_else(|| anyhow!("No linting script configured"))?;

    let shebang = Shebang::from(config.shebang().clone());
    let pname = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;

    let bar = progressbars.bar();
    bar.set_message("Linting package scripts...");

    let lint_results = repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
        .map(|pkg| {
            let shebang = shebang.clone();
            let bar = bar.clone();
            async move {
                trace!("Linting script of {} {} with '{}'", pkg.name(), pkg.version(), linter.display());
                let cmd = tokio::process::Command::new(linter);
                let script = ScriptBuilder::new(&shebang)
                    .build(pkg, config.available_phases(), *config.strict_script_interpolation())?;

                let (status, stdout, stderr) = script.lint(cmd).await?;
                bar.inc(1);
                Ok((pkg.name().clone(), pkg.version().clone(), status, stdout, stderr))
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?
        .into_iter()
        .map(|tpl| {
            let pkg_name = tpl.0;
            let pkg_vers = tpl.1;
            let status = tpl.2;
            let stdout = tpl.3;
            let stderr = tpl.4;

            if status.success() {
                info!("Linting {pkg_name} {pkg_vers} script (exit {status}):\nstdout:\n{stdout}\n\nstderr:\n\n{stderr}",
                    pkg_name = pkg_name,
                    pkg_vers = pkg_vers,
                    status = status,
                    stdout = stdout,
                    stderr = stderr
                );
                true
            } else {
                error!("Linting {pkg_name} {pkg_vers} errored ({status}):\n\nstdout:\n{stdout}\n\nstderr:\n{stderr}\n\n",
                    pkg_name = pkg_name,
                    pkg_vers = pkg_vers,
                    status = status,
                    stdout = stdout,
                    stderr = stderr
                );
                false
            }
        })
        .collect::<Vec<_>>();

    let lint_ok = lint_results.iter().all(|b| *b);

    if !lint_ok {
        bar.finish_with_message("Linting errored");
        return Err(anyhow!("Linting was not successful"))
    } else {
        bar.finish_with_message(&format!("Finished linting {} package scripts", lint_results.len()));
        Ok(())
    }
}

