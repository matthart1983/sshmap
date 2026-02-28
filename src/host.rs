use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    pub identity_file: Option<String>,
    pub group: String,
    #[serde(skip)]
    pub status: HostStatus,
}

#[derive(Debug, Clone, Default)]
pub enum HostStatus {
    #[default]
    Unknown,
    Checking,
    Up(f64),   // rtt ms
    Down,
}

impl Host {
    pub fn status_label(&self) -> &str {
        match &self.status {
            HostStatus::Unknown => "?",
            HostStatus::Checking => "...",
            HostStatus::Up(_) => "UP",
            HostStatus::Down => "DOWN",
        }
    }

    pub fn rtt_label(&self) -> String {
        match &self.status {
            HostStatus::Up(rtt) => format!("{:.0}ms", rtt),
            _ => "—".to_string(),
        }
    }

    pub fn ssh_command(&self) -> Vec<String> {
        let mut args = vec!["ssh".to_string()];
        if self.port != 22 {
            args.push("-p".to_string());
            args.push(self.port.to_string());
        }
        if let Some(ref key) = self.identity_file {
            args.push("-i".to_string());
            args.push(key.clone());
        }
        if !self.user.is_empty() {
            args.push(format!("{}@{}", self.user, self.hostname));
        } else {
            args.push(self.hostname.clone());
        }
        args
    }
}

pub fn load_hosts() -> Vec<Host> {
    let mut hosts = Vec::new();

    // 1. Parse ~/.ssh/config
    hosts.extend(parse_ssh_config());

    // 2. Load sshmap's own config (overrides/supplements)
    if let Some(extra) = load_sshmap_config() {
        for h in extra {
            // Don't duplicate aliases already from ssh config
            if !hosts.iter().any(|existing| existing.alias == h.alias) {
                hosts.push(h);
            }
        }
    }

    // Sort by group then alias
    hosts.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then(a.alias.cmp(&b.alias))
    });

    hosts
}

fn parse_ssh_config() -> Vec<Host> {
    let home = dirs_home();
    let config_path = home.join(".ssh").join("config");
    let content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut hosts = Vec::new();
    let mut current_alias: Option<String> = None;
    let mut hostname = String::new();
    let mut user = String::new();
    let mut port: u16 = 22;
    let mut identity: Option<String> = None;
    let mut group = String::from("default");

    for line in content.lines() {
        let trimmed = line.trim();

        // Comments with group tags: # group: production
        if let Some(tag) = trimmed.strip_prefix('#') {
            let tag = tag.trim();
            if let Some(g) = tag.strip_prefix("group:") {
                group = g.trim().to_string();
            }
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            continue;
        }

        let key = parts[0].to_lowercase();
        let val = parts[1].trim().to_string();

        match key.as_str() {
            "host" => {
                // Save previous host
                if let Some(alias) = current_alias.take() {
                    if !alias.contains('*') && !alias.contains('?') {
                        let h = hostname.clone();
                        hosts.push(Host {
                            alias: alias.clone(),
                            hostname: if h.is_empty() { alias } else { h },
                            user: user.clone(),
                            port,
                            identity_file: identity.clone(),
                            group: group.clone(),
                            status: HostStatus::Unknown,
                        });
                    }
                }
                current_alias = Some(val);
                hostname.clear();
                user.clear();
                port = 22;
                identity = None;
            }
            "hostname" => hostname = val,
            "user" => user = val,
            "port" => port = val.parse().unwrap_or(22),
            "identityfile" => {
                let expanded = val.replace('~', &dirs_home().to_string_lossy());
                identity = Some(expanded);
            }
            _ => {}
        }
    }

    // Don't forget the last host
    if let Some(alias) = current_alias {
        if !alias.contains('*') && !alias.contains('?') {
            let h = hostname;
            hosts.push(Host {
                alias: alias.clone(),
                hostname: if h.is_empty() { alias } else { h },
                user,
                port,
                identity_file: identity,
                group,
                status: HostStatus::Unknown,
            });
        }
    }

    hosts
}

fn sshmap_config_path() -> PathBuf {
    dirs_home().join(".config").join("sshmap").join("hosts.json")
}

fn load_sshmap_config() -> Option<Vec<Host>> {
    let path = sshmap_config_path();
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_sshmap_config(hosts: &[Host]) -> anyhow::Result<()> {
    let path = sshmap_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(hosts)?;
    fs::write(&path, json)?;
    Ok(())
}

pub fn create_sample_config() -> anyhow::Result<()> {
    let path = sshmap_config_path();
    if path.exists() {
        return Ok(());
    }

    let sample = vec![
        Host {
            alias: "web-prod-1".into(),
            hostname: "192.168.1.10".into(),
            user: "deploy".into(),
            port: 22,
            identity_file: None,
            group: "production".into(),
            status: HostStatus::Unknown,
        },
        Host {
            alias: "web-staging".into(),
            hostname: "192.168.1.20".into(),
            user: "deploy".into(),
            port: 22,
            identity_file: None,
            group: "staging".into(),
            status: HostStatus::Unknown,
        },
        Host {
            alias: "db-prod".into(),
            hostname: "192.168.1.30".into(),
            user: "admin".into(),
            port: 2222,
            identity_file: None,
            group: "production".into(),
            status: HostStatus::Unknown,
        },
        Host {
            alias: "dev-box".into(),
            hostname: "10.0.0.5".into(),
            user: "matt".into(),
            port: 22,
            identity_file: None,
            group: "dev".into(),
            status: HostStatus::Unknown,
        },
    ];

    save_sshmap_config(&sample)?;
    Ok(())
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
