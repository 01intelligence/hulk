use std::fmt;

#[derive(Clone, Eq, PartialEq)]
pub struct Host {
    pub name: String,
    pub port: Option<u16>,
}

impl Host {
    pub fn new(name: String, port: Option<u16>) -> Host {
        Host { name, port }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.port {
            None => write!(f, "{}", self.name),
            Some(port) => {
                // We assume that host is a literal IPv6 address
                // if host has colons.
                if self.name.find(':').is_some() {
                    write!(f, "[{}]:{}", self.name, port)
                } else {
                    write!(f, "{}:{}", self.name, port)
                }
            }
        }
    }
}

// TODO: parse, json serialize/deserialize
