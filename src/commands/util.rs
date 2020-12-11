use std::path::Path;

use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use log::{error, info, trace};
use tokio::stream::StreamExt;

use crate::config::*;
use crate::package::Shebang;
use crate::package::Package;
use crate::package::ScriptBuilder;

pub fn getbool(m: &ArgMatches, name: &str, cmp: &str) -> bool {
    // unwrap is safe here because clap is configured with default values
    m.values_of(name).unwrap().any(|v| v == cmp)
}

pub async fn lint_packages<'a, I>(iter: I, linter: &Path, config: &Configuration, bar: indicatif::ProgressBar) -> Result<()>
    where I: Iterator<Item = &'a Package> + 'a
{
    let shebang = Shebang::from(config.shebang().clone());
    let lint_results = iter
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

