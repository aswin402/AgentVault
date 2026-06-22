use crate::cli::SearchArgs;
use anyhow::Result;

pub async fn handle(args: SearchArgs, _vault_dir_override: Option<&str>) -> Result<()> {
    println!("Command 'search' is not yet implemented (args: {:?})", args);
    Ok(())
}
