use crate::cli::InstallArgs;
use anyhow::Result;

pub async fn handle(args: InstallArgs) -> Result<()> {
    println!("Command 'install' is not yet implemented (args: {:?})", args);
    Ok(())
}
