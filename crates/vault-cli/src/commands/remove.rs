use crate::cli::RemoveArgs;
use anyhow::Result;

pub async fn handle(args: RemoveArgs, _vault_dir_override: Option<&str>) -> Result<()> {
    println!("Command 'remove' is not yet implemented (args: {:?})", args);
    Ok(())
}
