use crate::cli::ListArgs;
use anyhow::Result;

pub async fn handle(args: ListArgs) -> Result<()> {
    println!("Command 'list' is not yet implemented (args: {:?})", args);
    Ok(())
}
