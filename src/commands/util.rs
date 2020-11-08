use clap_v3::ArgMatches;

pub fn getbool(m: &ArgMatches, name: &str, cmp: &str) -> bool {
    // unwrap is safe here because clap is configured with default values
    m.values_of(name).unwrap().any(|v| v == cmp)
}

