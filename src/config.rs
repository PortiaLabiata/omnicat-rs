use std::collections::HashSet;
use utl::{frame::BoxEndpoint, udp::EndpointUdp};
use tokio::net::{UdpSocket};
use tokio::io;

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointKind {
    Udp,
    Tcp,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointRole {
    Server,
    Client,
}

#[derive(serde::Deserialize)]
pub struct EndpointConfig {
    kind: EndpointKind,
    name: String,
    role: EndpointRole,
    addr: String,

    #[serde(default = "bufsize_default")]
    bufsize: usize,

    to: Vec<String>,
}

fn bufsize_default() -> usize {
    return 1500;
}

#[derive(serde::Deserialize)]
pub struct Config {
    pub configs: Vec<EndpointConfig>,
}

pub enum ConfigError {
    RepeatingName(String),
    NonExistingDestination(String),
    Loop(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ConfigError::*;

        match self {
            RepeatingName(s) 
                => write!(f, "repeating name '{}'", s),
            NonExistingDestination(s) 
                => write!(f, "non-existent destination '{}'", s),
            Loop(s)
                => write!(f, "endpoint is itself's destination: '{}'", s),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut names = HashSet::new();

        for config in self.configs.iter() {
            let name = &config.name;
            if names.contains(name) {
                return Err(ConfigError::RepeatingName(name.clone()));
            }
            names.insert(name); 

            for dst in config.to.iter() {
                if dst == name {
                    return Err(ConfigError::Loop(name.clone()));
                }
            }
        }

        for config in self.configs.iter() {
            for dst in config.to.iter() {
                if !names.contains(dst) {
                    return Err(ConfigError::NonExistingDestination(dst.clone()));
                }
            }
        }

        Ok(())
    }
}

impl EndpointConfig {
    pub async fn create(&self) -> io::Result<BoxEndpoint> {
        match self.kind {
            EndpointKind::Udp => self.create_udp().await,
            EndpointKind::Tcp => todo!("Unimplemented"),
        }
    }

    pub fn name(&self) -> &str {
        return &self.name;
    }

    async fn create_udp(&self) -> io::Result<BoxEndpoint> {
        let sock;
        match self.role {
            EndpointRole::Server => {
                sock = UdpSocket::bind(&self.addr).await;
            },

            EndpointRole::Client => {
                sock = UdpSocket::bind("0.0.0.0:0").await;
            },
        };

        if let Err(e) = sock {
            return Err(e);
        }

        let sock = sock.unwrap();
        Ok(Box::pin(EndpointUdp::new(sock, self.bufsize)))
    }
}
