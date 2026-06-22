use crate::cli::DoctorArgs;
use anyhow::Result;

pub async fn handle(args: DoctorArgs) -> Result<()> {
    println!("Command 'doctor' is not yet implemented (args: {:?})", args);
    Ok(())
}
