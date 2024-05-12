use std::io::{self, Write};
mod credentials;
mod drive;
mod encryption;
use drive::Drive;

#[tokio::main]
async fn main() {
    let credential_manager = credentials::CredentialManager {
        directory: format!("{}/.cache/cloudencryptor/", std::env::var("HOME").unwrap()),
    };
    let mut dr = Drive::login(&credential_manager).await;


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
            "l" => dr = Drive::new_login(&credential_manager).await,
            "f" => dr.list_files().await,
            _ => {
                println!("f - list files");
                println!("u - upload file");
                println!("d - download file");
                println!("l - quit");
                println!("l - log in");
            }
        }
    }
}
