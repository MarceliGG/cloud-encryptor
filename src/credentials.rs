use std::fs;
use std::io::Write;

use aes_gcm::aes::cipher::typenum::marker_traits;

pub struct CredentialReadError;

pub struct CredentialManager {
    pub directory: String,
}

impl CredentialManager {
    pub fn get_token(&self) -> Result<String, CredentialReadError> {
        match fs::read_to_string(format!("{}/token", &self.directory)) {
            Ok(c) => return Ok(c),
            Err(_) => return Err(CredentialReadError),
        }
    }

    pub fn get_passwd(&self) -> Result<String, CredentialReadError> {
        match fs::read_to_string(format!("{}/key", &self.directory)) {
            Ok(c) => return Ok(c),
            Err(_) => return Err(CredentialReadError),
        }
    }

    pub fn save_token(&self, token: String) {
        match fs::write(format!("{}/token", &self.directory), token) {
            Err(e) => println!("Could not write file: {}", e),
            Ok(_) => println!("Saved refresh new token."),
        }
    }

    pub fn save_passwd(&self, passwd: &str) {
        match fs::write(format!("{}/token", &self.directory), passwd) {
            Err(e) => println!("Could not write file: {}", e),
            Ok(_) => println!("Saved password."),
        }
    }
}
