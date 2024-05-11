use std::fs;
use std::io::Write;

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

    pub fn get_passwd(&self) -> Result<Vec<u8>, CredentialReadError> {
        match fs::read(format!("{}/key", &self.directory)) {
            Ok(c) => return Ok(c),
            Err(_) => return Err(CredentialReadError),
        }
    }

    pub fn save_token(&self, token: String) {
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&format!("{}/token", &self.directory));

        match file {
            Ok(mut f) => {
                f.write_all(&token.as_bytes());
            }
            Err(e) => {
                println!("Could not write file: {}", e);
            }
        }
    }

    pub fn save_passwd(&self, key: Vec<u8>) {
        let file_key = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&format!("{}/key", &self.directory));

        match file_key {
            Ok(mut f) => {
                f.write_all(&key);
            }
            Err(e) => {
                println!("Could not write file: {}", e);
            }
        }
    }
}
