use clap_v3 as clap;
use clap::App;
use clap::Arg;
use clap::crate_authors;
use clap::crate_version;

pub fn cli<'a>() -> App<'a> {
    App::new("butido")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Generic Build Orchestration System for building linux packages with docker")

        .subcommand(App::new("build")
            .about("Build packages in containers")

            .arg(Arg::with_name("package_name")
                .required(true)
                .multiple(false)
                .index(1)
            )
            .arg(Arg::with_name("package_version")
                .required(false)
                .multiple(false)
                .index(2)
            )

            .arg(Arg::with_name("env")
                .required(false)
                .multiple(true)
                .short('E')
                .long("env")
                .validator(env_pass_validator)
                .help("Pass these variables to each build job (expects \"key=value\" or name of variable available in ENV)")
            )

            .arg(Arg::with_name("image")
                .required(true)
                .multiple(false)
                .short('I')
                .long("image")
                .help("Name of the docker image to use")
            )

            .arg(Arg::with_name("overwrite_release_dir")
                .required(true)
                .multiple(false)
                .long("realease-dir")
                .help("Overwrite the release directory. This is not recommended. Use the config file instead.")
            )

            .arg(Arg::with_name("overwrite_staging_dir")
                .required(true)
                .multiple(false)
                .short('S')
                .long("staging-dir")
                .help("Overwrite the staging directory.")
            )
        )

}

/// Naive check whether 's' is a 'key=value' pair or an existing environment variable
///
/// TODO: Clean up this spaghetti code
fn env_pass_validator(s: String) -> Result<(), String> {
    let v = s.split("=").collect::<Vec<_>>();

    if v.len() != 2 {
        if v.len() == 1 {
            if let Some(name) = v.get(0) {
                match std::env::var(name) {
                    Err(std::env::VarError::NotPresent) => {
                        return Err(format!("Environment variable '{}' not present", name))
                    },
                    Err(std::env::VarError::NotUnicode(_)) => {
                        return Err(format!("Environment variable '{}' not unicode", name))
                    },
                    Ok(_) => return Ok(()),
                }
            } else {
                return Err(format!("BUG")) // TODO: Make nice, not runtime error
            }
        } else {
            return Err(format!("Expected a 'key=value' string, got something different: '{}'", s))
        }
    } else {
        if let Some(key) = v.get(0) {
            if key.chars().any(|c| c == ' ' || c == '\t' || c == '\n') {
                return Err(format!("Invalid characters found in key: '{}'", s))
            }
        } else {
            return Err(format!("No key found in '{}'", s))
        }

        if let Some(value) = v.get(1) {
            if value.chars().any(|c| c == ' ' || c == '\t' || c == '\n') {
                return Err(format!("Invalid characters found in value: '{}'", s))
            }
        } else {
            return Err(format!("No value found in '{}'", s))
        }
    }

    Ok(())
}

