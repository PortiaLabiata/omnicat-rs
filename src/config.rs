use std::collections::HashSet;

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointKind {
    Udp,
    Tcp,
}

#[derive(serde::Deserialize)]
pub struct EndpointConfig {
    kind: EndpointKind,
    name: String,

    #[serde(default = "addr_default")]
    addr: String,

    #[serde(default = "port_default")]
    port: u16,

    to: Vec<String>,
}

fn addr_default() -> String {
    return "0.0.0.0".to_string();
}

fn port_default() -> u16 {
    return 0;
}

#[derive(serde::Deserialize)]
pub struct Config {
    configs: Vec<EndpointConfig>,
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
