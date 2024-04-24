use onedrive_api::{
    Auth, ClientCredential, DriveLocation, FileName, ItemLocation, OneDrive, Permission, Tenant,
};
use rouille::{router, Response, Server};
use std::fs;
use std::io::{self, Write};

static mut SERVER_STOP: bool = false;
static mut CODE_G: String = String::new();

#[tokio::main]
async fn main() {
    let cache_file_path = format!(
        "{}/.cache/cloudencryptor/token",
        std::env::var("HOME").unwrap()
    );

    let token: String;
    print!("Do you want to log in with new account? [y/N]");
    let mut input: String = String::new();
    let _ = io::stdout().flush();
    io::stdin()
        .read_line(&mut input)
        .expect("Error reading from STDIN");

    match input.as_str().trim() {
        "y" => token = login(cache_file_path).await,
        _ => token = fs::read_to_string(cache_file_path).unwrap(),
    }

    let drive = OneDrive::new(token, DriveLocation::me());

    let folder_item = drive
        .create_folder(ItemLocation::root(), FileName::new("test_folder").unwrap())
        .await;
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
                    CODE_G = request.get_param("code").unwrap();
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
    let code;
    unsafe {
        code = CODE_G.clone();
    }
    let token_response = auth
        .login_with_code(&code, &ClientCredential::None)
        .await
        .unwrap();
    let token = token_response.access_token;
    let path = std::path::Path::new(&cache_file);
    let prefix = path.parent().unwrap();
    fs::create_dir_all(prefix).unwrap();
    let mut file = fs::File::create(cache_file).unwrap();
    file.write_all(token.as_bytes());
    token
}
