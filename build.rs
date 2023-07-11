use anyhow::Result;
use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        .build_timestamp()
        .cargo_debug()
        .git_sha(false)
        .git_commit_timestamp()
        .git_describe(true, true, None)
        .emit()?;
    Ok(())
}
