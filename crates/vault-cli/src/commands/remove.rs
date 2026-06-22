use crate::cli::RemoveArgs;
use anyhow::Result;

pub async fn handle(args: RemoveArgs) -> Result<()> {
    println!("Command 'remove' is not yet implemented (args: {:?})", args);
    Ok(())
}
