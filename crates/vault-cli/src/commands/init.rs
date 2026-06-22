use crate::cli::InitArgs;
use anyhow::Result;

pub async fn handle(args: InitArgs) -> Result<()> {
    println!("Command 'init' is not yet implemented (args: {:?})", args);
    Ok(())
}
