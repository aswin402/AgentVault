use crate::cli::ListArgs;
use anyhow::Result;

pub async fn handle(args: ListArgs, _vault_dir_override: Option<&str>) -> Result<()> {
    println!("Command 'list' is not yet implemented (args: {:?})", args);
    Ok(())
}
