use anyhow::Result;
use vergen::EmitBuilder;
use std::error::Error;


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
