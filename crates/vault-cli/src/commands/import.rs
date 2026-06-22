use crate::cli::ImportArgs;
use anyhow::Result;

pub async fn handle(args: ImportArgs, _vault_dir_override: Option<&str>) -> Result<()> {
    println!("Command 'import' is not yet implemented (args: {:?})", args);
    Ok(())
}
