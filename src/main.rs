use aes_gcm::{
    aead::{KeyInit, OsRng},
    Aes256Gcm,
};
use std::io::{self, Write};
mod credentials;
mod drive;

#[tokio::main]
async fn main() {
    let credential_manager = credentials::CredentialManager {
        directory: format!("{}/.cache/cloudencryptor/", std::env::var("HOME").unwrap()),
    };
    let token: String;
    match credential_manager.get_token() {
        Ok(t) => {
            token = t;
            println!("Using exsiting token.");
        }
        Err(_) => {
            token = drive::Drive::login().await;
            credential_manager.save_token(token.clone());
        }
    }

    let key;
    match credential_manager.get_passwd() {
        Ok(k) => {
            key = k;
            println!("Using exsiting encryption key.")
        }
        Err(_) => {
            key = Aes256Gcm::generate_key(OsRng).to_vec();
            credential_manager.save_passwd(key.clone());
            println!("Generated key: {:?}", key)
        }
    }
    let mut dr = drive::Drive::new(token, key);

    loop {
        print!(">>");
        let mut input: String = String::new();
        let _ = io::stdout().flush();
        io::stdin()
            .read_line(&mut input)
            .expect("Error reading from STDIN");

        match input.as_str().trim() {
            "u" => {
                print!("File path: ");
                let mut input: String = String::new();
                let _ = io::stdout().flush();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Error reading from STDIN");
                dr.upload(input.trim().to_string()).await;
            }
            "d" => {
                print!("File name: ");
                let mut input: String = String::new();
                let _ = io::stdout().flush();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Error reading from STDIN");
                dr.download(input.trim().to_string()).await;
            }
            "q" => break,
            "l" => {
                let token = drive::Drive::login().await;
                credential_manager.save_token(token.clone());
                let key;
                match credential_manager.get_passwd() {
                    Ok(k) => {
                        key = k;
                        println!("Using exsiting encryption key.")
                    }
                    Err(_) => {
                        key = Aes256Gcm::generate_key(OsRng).to_vec();
                        credential_manager.save_passwd(key.clone());
                        println!("Generated key: {:?}", key)
                    }
                }
                let mut dr = drive::Drive::new(token, key);
            }
            "help" | "h" => {
                println!("h - help menu");
                println!("l - log in");
                println!("u - upload file");
                println!("f - list files");
                println!("d - download file");
            }
            "f" => dr.list_files().await,
            _ => println!("err"),
        }
    }
}
