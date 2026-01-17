use crate::http::{HeaderKey, Request};
use crate::routing::Route;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::io;
use std::io::Error;
use std::io::ErrorKind::InvalidData;
use std::net::IpAddr;
use std::path::Path;
use tracing::warn;

#[derive(Deserialize)]
pub struct Auth {
    #[serde(default)]
    pub users: Option<HashMap<String, String>>,
    #[serde(default)]
    pub api_keys: Option<HashSet<String>>,
}

#[derive(Deserialize)]
pub struct AccessControl {
    #[serde(default)]
    pub whitelist: Option<HashSet<IpAddr>>,
    #[serde(default)]
    pub auth: Option<Auth>,
}

#[derive(Deserialize)]
pub struct Config {
    pub listen: String,
    pub routes: Vec<Route>,
    #[serde(default)]
    pub access: Option<AccessControl>
}

impl Config {
    pub fn new(config: &String) -> io::Result<Self> {
        match Path::new(&config).extension().map(|ext| ext.to_ascii_lowercase()) {
            Some(ext) => {
                let data = read_to_string(&config).map_err(|e| Error::new(InvalidData, format!("Failed to load config file {config}: {e}")))?;
                match ext.to_ascii_lowercase().to_str() {
                    Some("toml") => toml::from_str(&data).map_err(|e| Error::new(InvalidData, e)),
                    Some("yaml") | Some("yml") => serde_yaml::from_str(&data).map_err(|e| Error::new(InvalidData, e)),
                    _ => Err(Error::new(InvalidData, format!("Unsupported config extension {:?}", ext))),
                }
            },
            _ => Err(Error::new(InvalidData, "Config extension missing [use .toml, .yaml or .yml"))
        }
    }

    pub fn add_routes(&mut self, routes: Vec<Route>) {
        self.routes.extend(routes);
    }

    pub fn permit_addr(&self, peer_addr: IpAddr) -> bool {
        self.access.as_ref().map_or(true, |acc| acc.whitelist.as_ref().map_or(true, |ips| ips.contains(&peer_addr)))
    }

    pub fn permit_api_key(&self, api_key: &[u8]) -> bool {
        self.access.as_ref()
            .map_or(true, |acc| acc.auth.as_ref()
                .map_or(true, |auth| auth.api_keys.as_ref()
                    .map_or(false, |keys| str::from_utf8(api_key).ok().map_or(false, |key| keys.contains(key)))))
    }

    pub fn permit_user(&self, header: &[u8]) -> bool {
        if let Some(users) = self.access.as_ref().and_then(|acc| acc.auth.as_ref().and_then(|auth| auth.users.as_ref())) {
            return if header.len() > 5 && header.get(..5).map_or(false, |h| h.eq_ignore_ascii_case(b"Basic")) {
                let header_value = match header.get(5..).map(|x| BASE64_STANDARD.decode(x.trim_ascii_start())) {
                    Some(Ok(v)) => v,
                    _ => return false,
                };
                let mut split = header_value.splitn(2, |x| *x == b':');
                let u = split.next().map(|user| str::from_utf8(user).ok()).flatten();
                let p = split.next();
                let result = u.is_some() && p.is_some() && u.and_then(|user| users.get(user)).map(|x| x.as_bytes()) == p;
                if !result {
                    warn!("Basic authentication failed for user {}", u.unwrap_or(""));
                }
                result
            } else {
                false
            }
        }
        true
    }

    pub fn select_upstream(&self, request: &Request) -> Option<&Route> {
        self.routes.iter().find(|r| {
            request.headers.get(&HeaderKey("host".as_bytes())).map(|h| *h == r.host).unwrap_or(false) &&
                request.path.starts_with(&r.path)
        })
    }
    
    pub fn is_proxy_private(&self) -> bool {
        self.access.as_ref().and_then(|acc| acc.auth.as_ref()).is_some()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn config_permit(users: Option<(&str, &str)>, api_keys: Option<&str>) -> Config {
        Config {
            listen: "".to_string(),
            routes: vec![],
            access: Some(
                AccessControl {
                    whitelist: None,
                    auth: Some(
                        Auth {
                            users: users.map(|u| [(u.0.to_string(), u.1.to_string())].into_iter().collect::<HashMap<String, String>>()),
                            api_keys: api_keys.map(|key| [key].iter().map(|x| x.to_string()).collect::<HashSet<String>>()),
                        }
                    )
                }
            )
        }
    }

    #[test]
    fn test_auth_missing_colon() {
        let header = "adminpassword";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "password")), None);

        assert!(!config.permit_user(format!("Basic {encoded}").as_bytes()));
    }

    #[test]
    fn test_no_access() {
        let config = Config {
            listen: "".to_string(),
            routes: vec![],
            access: None,
        };

        assert!(config.permit_user(b"Basic 123"));
        assert!(config.permit_addr(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(config.permit_api_key(b"api_key"));
    }

    #[test]
    fn test_no_auth_no_whitelist() {
        let config = Config {
            listen: "".to_string(),
            routes: vec![],
            access: Some(AccessControl {
                whitelist: None,
                auth: None,
            }),
        };

        assert!(config.permit_user(b"Basic 123"));
        assert!(config.permit_addr(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(config.permit_api_key(b"api_key"));
    }

    #[test]
    fn test_auth_valid_credentials() {
        let header = "admin:password";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "password")), None);

        assert!(config.permit_user(format!("Basic {encoded}").as_bytes()));
    }

    #[test]
    fn test_auth_empty_password() {
        let header = "admin:";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "")), None);

        assert!(config.permit_user(format!("Basic {encoded}").as_bytes()));
    }

    #[test]
    fn test_auth_whitespace_resilience() {
        let header = "admin:";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "")), None);

        assert!(config.permit_user(format!("Basic    {encoded}").as_bytes()));
    }

    #[test]
    fn test_auth_incorrect_password() {
        let header = "admin:password1";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "password")), None);

        assert!(!config.permit_user(format!("Basic {encoded}").as_bytes()));
    }

    #[test]
    fn test_auth_password_with_colon() {
        let header = "admin:pass:word";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "pass:word")), None);

        assert!(config.permit_user(format!("Basic {encoded}").as_bytes()));
    }

    #[test]
    fn test_auth_case_insensitivity() {
        let header = "admin:password";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "password")), None);

        assert!(config.permit_user(format!("bASIc {encoded}").as_bytes()));
    }

    #[test]
    fn test_auth_non_utf8_password() {
        let header = b"admin:\xff\xfe\xfd";
        let encoded = BASE64_STANDARD.encode(header);
        let config = config_permit(Some(("admin", "password")), None);

        assert!(!config.permit_user(format!("Basic {encoded}").as_bytes()));
    }

    #[test]
    fn test_auth_malformed_basic() {
        let config = config_permit(Some(("admin", "password")), None);

        assert!(!config.permit_user(b"Basic 2t34"))
    }

    #[test]
    fn test_auth_empty_value() {
        let config = config_permit(Some(("admin", "password")), None);

        assert!(!config.permit_user(b""))
    }

    #[test]
    fn test_auth_invalid_characters() {
        let config = config_permit(Some(("admin", "password")), None);

        assert!(!config.permit_user("Basic admłinółśą".as_bytes()))
    }

    #[test]
    fn test_auth_missing_basic() {
        let config = config_permit(Some(("admin", "password")), None);

        assert!(!config.permit_user(b"x"))
    }

    #[test]
    fn test_no_api_key() {
        let config = config_permit(None, None);

        assert!(!config.permit_api_key(b"1234"));
    }

    #[test]
    fn test_api_key_empty() {
        let config = Config {
            listen: "".to_string(),
            routes: vec![],
            access: Some(
                AccessControl {
                    whitelist: None,
                    auth: Some(
                        Auth {
                            users: None,
                            api_keys: Some(HashSet::new()),
                        }
                    )
                }
            )
        };

        assert!(!config.permit_api_key(b"1234"));
    }

    #[test]
    fn test_api_key_invalid() {
        let config = config_permit(None, Some("123"));

        assert!(!config.permit_api_key(b"1234"));
    }

    #[test]
    fn test_api_key_not_utf8() {
        let config = config_permit(None, Some("1234"));
        assert!(!config.permit_api_key(b"\xff\xfe\xfd"));
    }

    #[test]
    fn test_api_key_valid() {
        let config = config_permit(None, Some("1234"));

        assert!(config.permit_api_key(b"1234"));
    }

    #[test]
    fn test_whitelist_empty() {
        let config = Config {
            listen: "".to_string(),
            routes: vec![],
            access: Some(AccessControl {
                whitelist: Some(HashSet::new()),
                auth: None,
            }),
        };

        assert!(!config.permit_addr(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1))));
    }

    #[test]
    fn test_whitelist_invalid_addr() {
        let config = Config {
            listen: "".to_string(),
            routes: vec![],
            access: Some(AccessControl {
                whitelist: Some(HashSet::from([IpAddr::from(Ipv4Addr::new(127, 0, 0, 2))])),
                auth: None,
            }),
        };

        assert!(!config.permit_addr(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1))));
    }

    #[test]
    fn test_whitelist_valid_addr() {
        let config = Config {
            listen: "".to_string(),
            routes: vec![],
            access: Some(AccessControl {
                whitelist: Some(HashSet::from([IpAddr::from(Ipv4Addr::new(127, 0, 0, 1))])),
                auth: None,
            }),
        };

        assert!(config.permit_addr(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1))));
    }

}