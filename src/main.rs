mod config;

use std::collections::HashMap;

use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
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

    let endpoint_num = config.configs.len();
    let mut endpoints = Vec::with_capacity(endpoint_num);

    let mut names_map = HashMap::with_capacity(endpoint_num);
    for (i, config) in config.configs.iter().enumerate() {
        names_map.insert(config.name(), i);
    }

    let mut destinations = HashMap::with_capacity(endpoint_num);
    for config_i in config.configs.iter() {
        let dests: Vec<_> = config_i
            .to()
            .iter()
            .map(|v| *names_map.get(v).unwrap())
            .collect();

        let &idx = names_map.get(&config_i.name()).unwrap();
        destinations.insert(idx, dests);
    }

    let mut creation_futures = FuturesUnordered::new();
    for config in config.configs.iter() {
        creation_futures.push(async move {
            config.create().await
                .map_err(|v| (v, config.name()))
        });
    }

    while let Some(r) = creation_futures.next().await {
        match r {
            Ok(v) => endpoints.push(v),
            Err((e, name)) => {
                eprintln!("Failed to create endpoint '{}': {}",
                    name, e);
                return Err(Error::from(ErrorKind::InvalidData));
            }
        }
    }

    loop {
        let (index, frame) = {
            let mut futures: FuturesUnordered<_> = endpoints
                .iter_mut()
                .enumerate()
                .map(|(i, v)| async move {
                    return (i, v.next().await);
                })
                .collect();

            match futures.next().await {
                Some(x) => x,
                None => continue,
            }
        };

        if frame.is_none() {
            continue;
        }

        let frame = frame.unwrap();
        if let Err(e) = frame {
            eprintln!("RX error: {}", e);
            break;
        }

        let frame = frame.unwrap();
        for &dest_index in destinations
            .get(&index)
            .expect("Failed to find existing endpoint")
        {
            let ep = &mut endpoints[dest_index];
            let _ = ep.feed(frame.clone()).await;
            let _ = ep.flush().await;
        }
    }

    Ok(())
}
