use crate::cli::SyncArgs;
use anyhow::Result;

pub async fn handle(args: SyncArgs) -> Result<()> {
    println!("Command 'sync' is not yet implemented (args: {:?})", args);
    Ok(())
}
