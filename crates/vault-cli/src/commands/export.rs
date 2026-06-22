use crate::cli::ExportArgs;
use anyhow::Result;

pub async fn handle(args: ExportArgs, _vault_dir_override: Option<&str>) -> Result<()> {
    println!("Command 'export' is not yet implemented (args: {:?})", args);
    Ok(())
}
