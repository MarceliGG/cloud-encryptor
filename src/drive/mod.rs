static mut SERVER_STOP: bool = false;
static mut CODE: String = String::new();
use std::io::Write;

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::Result;
use aes_gcm::aes::Aes256;
use aes_gcm::{
    aead::{AeadCore, AeadInPlace, KeyInit, OsRng},
    Aes256Gcm,
};
use onedrive_api::{
    Auth, ClientCredential, DriveLocation, ItemLocation, OneDrive, Permission, Tenant,
};
use rouille::{router, Response, Server};
use std::fs;
mod encryption;

pub struct Drive {
    drive: OneDrive,
    key: Vec<u8>,
}

impl Drive {
    pub fn new(token: String, key: Vec<u8>) -> Self {
        Drive {
            drive: OneDrive::new(token, DriveLocation::me()),
            key: key,
        }
    }

    async fn download_from_url(url: String) -> Vec<u8> {
        let mut resp = reqwest::get(url).await.expect("request failed");
        resp.bytes().await.unwrap().to_vec()
    }

    fn decrypt_and_save(data: Vec<u8>, file_name: String, key: &Vec<u8>) {
        let data_decrypted = encryption::decrypt(data, key);

        let mut path = format!(
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
                "{}/{}{}.{}",
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
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path_f);

        match file {
            Ok(mut f) => {
                f.write_all(&data_decrypted);
                println!("Downloaded to: {}", path_f);
            }
            Err(e) => {
                println!("Could not write file: {}", e);
            }
        }
    }

    pub async fn download(&self, file: String) {
        match self
            .drive
            .get_item_download_url(
                ItemLocation::from_path(&format!("/encrypted/{}", file)).unwrap(),
            )
            .await
        {
            Ok(l) => Drive::decrypt_and_save(Drive::download_from_url(l).await, file, &self.key),
            Err(e) => println!("{}", e),
        }
    }

    pub async fn list_files(&self) {
        let list = self
            .drive
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
                    .upload_small(
                        ItemLocation::from_path(&format!("/encrypted/{}", name)).unwrap(),
                        encryption::encrypt(content, &self.key),
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

    pub async fn login() -> String {
        let auth = Auth::new(
            "48100e01-0c50-4c12-8887-d3fa69416e02",
            Permission::new_read().write(true),
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
            match auth
                .login_with_code(&CODE.clone(), &ClientCredential::None)
                .await
            {
                Ok(a) => {
                    token_response = a.access_token
                }
                Err(e) => {
                    println!("{}", e);
                    token_response = String::from("a");
                }
            }
        }
        token_response
    }
}
