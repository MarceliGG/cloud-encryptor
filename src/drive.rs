static mut SERVER_STOP: bool = false;
static mut CODE: String = String::new();
use md5;
use std::io::{self, Write};

use crate::credentials::CredentialManager;
use crate::encryption;
use onedrive_api::{
    Auth, ClientCredential, DriveLocation, ItemLocation, OneDrive, Permission, Tenant,
};
use rouille::{router, Response, Server};
use std::fs;

fn get_key(credential_manager: &CredentialManager, new: bool) -> Vec<u8> {
    let pass;
    match credential_manager.get_passwd() {
        Ok(k) => {
            if new {
                print!("Do you want to use saved password? [y/N]: ");
                let mut input: String = String::new();
                let _ = io::stdout().flush();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Error reading from STDIN");
                match input.to_string().trim() {
                    "y" | "Y" => {
                        pass = k;
                        println!("Using exsiting encryption password.")
                    }
                    _ => {
                        print!("Enter password for encryption: ");
                        let mut input: String = String::new();
                        let _ = io::stdout().flush();
                        io::stdin()
                            .read_line(&mut input)
                            .expect("Error reading from STDIN");

                        pass = input;
                        credential_manager.save_passwd(&pass.clone());
                    }
                }
            } else {
                pass = k;
                println!("Using exsiting encryption password.")
            }
        }
        Err(_) => {
            print!("Enter password for encryption: ");
            let mut input: String = String::new();
            let _ = io::stdout().flush();
            io::stdin()
                .read_line(&mut input)
                .expect("Error reading from STDIN");

            pass = input;
            credential_manager.save_passwd(&pass.clone());
        }
    }

    format!("{:?}", md5::compute(pass)).as_bytes().to_vec()
}

pub struct Drive {
    drive: OneDrive,
    key: Vec<u8>,
}

impl Drive {
    pub async fn login(credential_manager: &CredentialManager) -> Self {
        let dr;
        match credential_manager.get_token() {
            Ok(refresh_token) => {
                match Auth::new(
                    "48100e01-0c50-4c12-8887-d3fa69416e02",
                    Permission::new_read().write(true).offline_access(true),
                    "http://127.0.0.1:3000/auth",
                    Tenant::Common,
                )
                .login_with_refresh_token(&refresh_token, &ClientCredential::None)
                .await
                {
                    Ok(token_response) => {
                        println!("Using exsiting refresh token.");
                        credential_manager.save_token(token_response.refresh_token.unwrap());
                        dr = Drive {
                            drive: OneDrive::new(token_response.access_token, DriveLocation::me()),
                            key: get_key(credential_manager, false),
                        }
                    }
                    Err(e) => {
                        println!("auth err {}", e);
                        dr = Drive::new_login(credential_manager).await;
                    }
                }
            }
            Err(_) => {
                println!("No refresh token. Please Log in...");
                dr = Drive::new_login(credential_manager).await
            }
        }
        dr
    }

    async fn download_from_url(url: String) -> Vec<u8> {
        let resp = reqwest::get(url).await.expect("request failed");
        resp.bytes().await.unwrap().to_vec()
    }

    fn decrypt_and_save(data: Vec<u8>, file_name: String, key: Vec<u8>) {
        let data_decrypted = encryption::decrypt(data, &key);

        let path = format!(
            "{}/{}",
            xdg_user::UserDirs::new()
                .unwrap()
                .downloads()
                .unwrap()
                .display(),
            file_name
        );
        let mut i = 0;
        let mut path_f = path.clone();
        while std::path::Path::new(&path_f).exists() {
            let p = std::path::Path::new(&path);
            path_f = format!(
                "{}/{}_{}.{}",
                xdg_user::UserDirs::new()
                    .unwrap()
                    .downloads()
                    .unwrap()
                    .display(),
                p.file_stem().unwrap().to_str().unwrap(),
                i,
                p.extension().unwrap().to_str().unwrap()
            );
            i += 1;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path_f);

        match file {
            Ok(mut f) => match f.write_all(&data_decrypted) {
                Ok(_) => println!("Downloaded to: {}", path_f),
                Err(e) => println!("Failed to write file: {}", e),
            },
            Err(e) => {
                println!("Could not write file: {}", e);
            }
        }
    }

    pub async fn download(&self, file: String) {
        match self
            .drive
            .clone()
            .get_item_download_url(
                ItemLocation::from_path(&format!("/encrypted/{}", file)).unwrap(),
            )
            .await
        {
            Ok(l) => {
                Drive::decrypt_and_save(Drive::download_from_url(l).await, file, self.key.clone())
            }

            Err(e) => println!("{}", e),
        }
    }

    pub async fn list_files(&self) {
        let list = self
            .drive
            .clone()
            .list_children(ItemLocation::from_path("/encrypted").unwrap())
            .await;
        match list {
            Ok(l) => {
                for i in l.iter() {
                    println!("{}", i.name.clone().unwrap());
                }
            }
            Err(e) => println!("{}", e),
        }
    }

    pub async fn upload(&self, file: String) {
        let path = std::path::Path::new(&file);
        let name = path.file_name().unwrap().to_str().unwrap();
        let file = fs::read(&file);
        match file {
            Ok(content) => {
                let r = self
                    .drive
                    .clone()
                    .upload_small(
                        ItemLocation::from_path(&format!("/encrypted/{}", name)).unwrap(),
                        encryption::encrypt(content, &self.key.clone()),
                    )
                    .await;
                match r {
                    Ok(_) => println!("uploaded"),
                    Err(e) => println!("err: {}", e),
                }
            }
            Err(_) => {
                println!("could not read file")
            }
        };
    }

    pub async fn new_login(credential_manager: &CredentialManager) -> Self {
        let auth = Auth::new(
            "48100e01-0c50-4c12-8887-d3fa69416e02",
            Permission::new_read().write(true).offline_access(true),
            "http://127.0.0.1:3000/auth",
            Tenant::Common,
        );

        println!("{}", auth.code_auth_url());

        let server = Server::new("127.0.0.1:3000", move |request| {
            let result = router!(request,
                (GET) (/auth) => {
                    unsafe {
                        CODE = request.get_param("code").unwrap();
                        SERVER_STOP = true;
                    }
                    "you can close this tab"
                },

                _ => "404"
            );
            Response::text(format!("{}", result))
        })
        .unwrap();
        loop {
            server.poll();
            unsafe {
                if SERVER_STOP {
                    break;
                }
            }
        }
        let token_response;
        unsafe {
            token_response = auth
                .login_with_code(&CODE.clone(), &ClientCredential::None)
                .await
                .unwrap();
        }
        credential_manager.save_token(token_response.refresh_token.clone().unwrap());
        Drive {
            drive: OneDrive::new(token_response.access_token, DriveLocation::me()),
            key: get_key(credential_manager, true),
        }
    }
}
