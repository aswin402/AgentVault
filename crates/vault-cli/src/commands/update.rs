use crate::cli::UpdateArgs;
use anyhow::Result;

pub async fn handle(args: UpdateArgs) -> Result<()> {
    println!("Command 'update' is not yet implemented (args: {:?})", args);
    Ok(())
}
