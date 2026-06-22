use crate::cli::StatusArgs;
use anyhow::Result;

pub async fn handle(args: StatusArgs) -> Result<()> {
    println!("Command 'status' is not yet implemented (args: {:?})", args);
    Ok(())
}
