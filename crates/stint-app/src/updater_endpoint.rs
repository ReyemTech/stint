#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

impl Channel {
    pub fn from_setting(value: &str) -> Self {
        match value {
            "beta" => Channel::Beta,
            _ => Channel::Stable,
        }
    }

    pub fn as_setting(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
        }
    }
}

pub fn resolve_endpoint(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => {
            "https://github.com/reyemtech/stint/releases/latest/download/latest.json"
        }
        Channel::Beta => {
            "https://github.com/reyemtech/stint/releases/download/beta-latest/latest.json"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_endpoint_is_releases_latest() {
        assert_eq!(
            resolve_endpoint(Channel::Stable),
            "https://github.com/reyemtech/stint/releases/latest/download/latest.json"
        );
    }

    #[test]
    fn beta_endpoint_is_beta_latest() {
        assert_eq!(
            resolve_endpoint(Channel::Beta),
            "https://github.com/reyemtech/stint/releases/download/beta-latest/latest.json"
        );
    }
}
