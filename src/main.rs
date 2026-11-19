mod config;

use tokio::io::{Error, ErrorKind};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Invalid number of arguments: config filename required");
        return Err(Error::from(ErrorKind::InvalidInput));
    }

    let config_file = std::fs::File::open(&args[1]);
    if let Err(e) = config_file {
        eprintln!("Failed to open file: {}", e);
        return Err(Error::from(ErrorKind::NotFound));
    }

    let config: Result<config::Config, _> = serde_json::from_reader(config_file.unwrap());
    if let Err(e) = config {
        eprintln!("Failed to load config: {}", e);
        return Err(Error::from(ErrorKind::InvalidData));
    }

    let config = config.unwrap();
    if let Err(e) = config.validate() {
        eprintln!("Failed to create endpoints: {}", e);
        return Err(Error::from(ErrorKind::InvalidData));
    }

    Ok(())
}
