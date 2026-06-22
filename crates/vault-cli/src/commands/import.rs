use crate::cli::ImportArgs;
use anyhow::Result;

pub async fn handle(args: ImportArgs) -> Result<()> {
    println!("Command 'import' is not yet implemented (args: {:?})", args);
    Ok(())
}
