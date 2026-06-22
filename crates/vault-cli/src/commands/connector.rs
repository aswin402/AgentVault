use crate::cli::{ConnectorArgs, ConnectorCommands};
use anyhow::Result;

pub async fn handle(args: ConnectorArgs) -> Result<()> {
    match args.command {
        ConnectorCommands::Add(subargs) => {
            println!("Command 'connector add' is not yet implemented (args: {:?})", subargs);
        }
        ConnectorCommands::List(subargs) => {
            println!("Command 'connector list' is not yet implemented (args: {:?})", subargs);
        }
        ConnectorCommands::Remove(subargs) => {
            println!("Command 'connector remove' is not yet implemented (args: {:?})", subargs);
        }
    }
    Ok(())
}
