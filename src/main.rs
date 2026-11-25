mod config;

use std::collections::HashMap;

use clap::Parser;
use futures::{SinkExt, StreamExt, stream::FuturesUnordered};
use tokio::io::{Error, ErrorKind};

#[derive(Parser, Debug)]
#[command(version = "1.0.0", about, long_about = None)]
struct Args {
    /// Config filename, defaults to omnicat.json
    name: String,

    /// Enable debug features?
    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let args = Args::parse();

    let config_file = std::fs::File::open(&args.name);
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

    let names_reverse_map: HashMap<_, _> = names_map
        .iter()
        .map(|(k, &v)| (v.clone(), k))
        .collect();

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
    for (i, config) in config.configs.iter().enumerate() {
        creation_futures.push(async move {
            config
                .create()
                .await
                .map_err(|v| (v, config.name()))
                .map(|v| (v, i))
        });
    }

    while let Some(r) = creation_futures.next().await {
        match r {
            Ok(v) => endpoints.push(v),
            Err((e, name)) => {
                eprintln!("Failed to create endpoint '{}': {}", name, e);
                return Err(Error::from(ErrorKind::InvalidData));
            }
        }
    }

    endpoints.sort_by(|a, b| a.1.cmp(&b.1));
    let mut endpoints: Vec<_> = endpoints.iter_mut().map(|(v, _)| v).collect();

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

            if args.debug {
                let source_name = &names_reverse_map[&index];
                let dest_name = &names_reverse_map[&dest_index];
                println!("FROM {} TO {}: {:?}", source_name, dest_name, frame.data);
            }

            let _ = ep.feed(frame.clone()).await;
            let _ = ep.flush().await;
        }
    }

    Ok(())
}
