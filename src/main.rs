use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aes::Aes256;
use onedrive_api::{
    Auth, ClientCredential, DriveLocation, ItemLocation, OneDrive, Permission, Tenant,
};
use rouille::{router, Response, Server};
use std::fs;
use std::io::{self, Write};

static mut SERVER_STOP: bool = false;
static mut CODE: String = String::new();

use aes_gcm::{
    aead::{AeadCore, AeadInPlace, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};

#[tokio::main]
async fn main() {
    let cache_dir = format!("{}/.cache/cloudencryptor/", std::env::var("HOME").unwrap());

    let mut token: String;
    let mut drive;

    match fs::read_to_string(format!("{}/token", cache_dir)) {
        Ok(t) => {
            token = t;
            println!("Using exsiting token.")
        }
        Err(_) => {
            token = login().await;
            cache_token(cache_dir.clone(), &token);
        }
    }

    drive = OneDrive::new(token, DriveLocation::me());

    let mut key;
    match fs::read(format!("{}/key", cache_dir)) {
        Ok(k) => {
            key = k;
            println!("Using exsiting encryption key.")
        }
        Err(_) => {
            key = Aes256Gcm::generate_key(OsRng).to_vec();
            cache_key(cache_dir.clone(), &key);
            println!("Generated key: {:?}", key)
        }
    }

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
                upload(input.trim().to_string(), &drive, &key).await;
            }
            "d" => {
                print!("File name: ");
                let mut input: String = String::new();
                let _ = io::stdout().flush();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Error reading from STDIN");
                download(input.trim().to_string(), &drive, &key).await;
            }
            "q" => break,
            "l" => {
                token = login().await;
                drive = OneDrive::new(&token, DriveLocation::me());
                cache_token(cache_dir.clone(), &token);
                match fs::read(format!("{}/key", cache_dir)) {
                    Ok(k) => {
                        key = k;
                        println!("Using exsiting encryption key.")
                    }
                    Err(_) => {
                        key = Aes256Gcm::generate_key(OsRng).to_vec();
                        cache_key(cache_dir.clone(), &key);
                        println!("Generated key: {:?}", key)
                    }
                }
            }
            "help" | "h" => {
                println!("h - help menu");
                println!("l - log in");
                println!("u - upload file");
                println!("f - list files");
                println!("d - download file");
            }
            "f" => list_files(&drive).await,
            _ => println!("err"),
        }
    }
}

fn cache_token(cache_dir: String, token: &String) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&format!("{}/token", cache_dir));

    match file {
        Ok(mut f) => {
            f.write_all(&token.as_bytes());
        }
        Err(e) => {
            println!("Could not write file: {}", e);
        }
    }
}

fn cache_key(cache_dir: String, key: &Vec<u8>) {
    let mut file_key = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&format!("{}/key", cache_dir));

    match file_key {
        Ok(mut f) => {
            f.write_all(&key);
        }
        Err(e) => {
            println!("Could not write file: {}", e);
        }
    }
}

fn decrypt(data: Vec<u8>, key: &Vec<u8>) -> Vec<u8> {
    let mut out = data;
    let nonce = GenericArray::from([
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
    ]);
    Aes256Gcm::new_from_slice(key)
        .unwrap()
        .decrypt_in_place(&nonce, b"", &mut out);
    out
}

fn encrypt(data: Vec<u8>, key: &Vec<u8>) -> Vec<u8> {
    let mut d = data;
    let n = &Aes256Gcm::generate_nonce(&mut OsRng);
    Aes256Gcm::new_from_slice(key)
        .unwrap()
        .encrypt_in_place(n, b"", &mut d);
    let mut out = n.to_vec();
    out.append(&mut d);
    out
}

async fn download_from_url(url: String) -> Vec<u8> {
    let mut resp = reqwest::get(url).await.expect("request failed");
    resp.bytes().await.unwrap().to_vec()
}

fn decrypt_and_save(data: Vec<u8>, file_name: String, key: &Vec<u8>) {
    let data_decrypted = decrypt(data, key);

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

async fn download(file: String, drive: &OneDrive, key: &Vec<u8>) {
    match drive
        .get_item_download_url(ItemLocation::from_path(&format!("/encrypted/{}", file)).unwrap())
        .await
    {
        Ok(l) => decrypt_and_save(download_from_url(l).await, file, key),
        Err(e) => println!("{}", e),
    }
}

async fn list_files(drive: &OneDrive) {
    let list = drive
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

async fn upload(file: String, drive: &OneDrive, key: &Vec<u8>) {
    let path = std::path::Path::new(&file);
    let name = path.file_name().unwrap().to_str().unwrap();
    let file = fs::read(&file);
    match file {
        Ok(content) => {
            let r = drive
                .upload_small(
                    ItemLocation::from_path(&format!("/encrypted/{}", name)).unwrap(),
                    encrypt(content, key),
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

async fn login() -> String {
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
        token_response = auth
            .login_with_code(&CODE.clone(), &ClientCredential::None)
            .await
            .unwrap();
    }
    token_response.access_token
}
