use onedrive_api::{
    Auth, ClientCredential, DriveLocation, FileName, ItemLocation, OneDrive, Permission, Result,
    Tenant,
};
use rouille::{router, Response, Server};
use std::fs;
use std::io::{self, Write};

static mut SERVER_STOP: bool = false;
static mut CODE: String = String::new();

#[tokio::main]
async fn main() {
    let cache_file_path = format!(
        "{}/.cache/cloudencryptor/token",
        std::env::var("HOME").unwrap()
    );

    let mut token: String;
    let mut drive;

    match fs::read_to_string(cache_file_path.clone()) {
        Ok(t) => {
            token = t;
        }
        Err(_) => token = login(cache_file_path.clone()).await,
    }

    drive = OneDrive::new(token, DriveLocation::me());

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
                upload(input.trim().to_string(), &drive).await;
            }
            "q" => break,
            "l" => {
                token = login(cache_file_path.clone()).await;
                drive = OneDrive::new(token, DriveLocation::me())
            }
            "help" | "h" => {
                println!("h - help menu");
                println!("l - log in");
                println!("u - upload file");
                println!("f - list files");
            }
            "f" => {
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
            _ => println!("err"),
        }
    }
}

async fn upload(file: String, drive: &OneDrive) {
    let path = std::path::Path::new(&file);
    let name = path.file_name().unwrap().to_str().unwrap();
    // println!("{}", file);
    let fc = fs::read_to_string(&file);
    match fc {
        Ok(c) => {
            let r = drive
                .upload_small(
                    ItemLocation::from_path(&format!("/encrypted/{}", name)).unwrap(),
                    c,
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
    }
}

async fn login(cache_file: String) -> String {
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
    let token = token_response.access_token;
    let path = std::path::Path::new(&cache_file);
    let prefix = path.parent().unwrap();
    fs::create_dir_all(prefix).unwrap();
    let mut file = fs::File::create(cache_file).unwrap();
    file.write_all(token.as_bytes());
    token
}
