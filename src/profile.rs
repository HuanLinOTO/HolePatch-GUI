use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub udp_mode: bool,
    pub bind_ip: String,
    pub bind_port: u16,
    pub stun_servers: Vec<String>,
    pub keepalive_host: String,
    pub keepalive_port: u16,
    pub forward_method: String,
    pub target_ip: String,
    pub target_port: u16,
    pub keepalive_interval: u64,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            name: "Default".into(),
            udp_mode: false,
            bind_ip: "0.0.0.0".into(),
            bind_port: 0,
            stun_servers: vec![],
            keepalive_host: String::new(),
            keepalive_port: 0,
            forward_method: "test".into(),
            target_ip: "0.0.0.0".into(),
            target_port: 0,
            keepalive_interval: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileStore {
    pub profiles: Vec<Profile>,
    pub last_used_index: Option<usize>,
}

impl ProfileStore {
    fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("holepatch");
        fs::create_dir_all(&config_dir).ok();
        config_dir.join("profiles.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            ProfileStore::default()
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(data) = serde_json::to_string_pretty(self) {
            fs::write(&path, data).ok();
        }
    }

    pub fn add_profile(&mut self, profile: Profile) {
        self.profiles.push(profile);
        self.save();
    }

    pub fn remove_profile(&mut self, index: usize) {
        if index < self.profiles.len() {
            self.profiles.remove(index);
            if let Some(last) = self.last_used_index {
                if last >= self.profiles.len() {
                    self.last_used_index = None;
                }
            }
            self.save();
        }
    }

    pub fn update_profile(&mut self, index: usize, profile: Profile) {
        if index < self.profiles.len() {
            self.profiles[index] = profile;
            self.save();
        }
    }

    pub fn set_last_used(&mut self, index: usize) {
        self.last_used_index = Some(index);
        self.save();
    }
}
