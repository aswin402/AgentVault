use crate::cli::InstallArgs;
use anyhow::Result;

pub async fn handle(args: InstallArgs, _vault_dir_override: Option<&str>) -> Result<()> {
    println!(
        "Command 'install' is not yet implemented (args: {:?})",
        args
    );
    Ok(())
}
