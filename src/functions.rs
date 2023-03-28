use std::process::exit;
use google_drive3::api::File;
use rand::seq::IteratorRandom;
use google_drive3::{oauth2, DriveHub};
use google_drive3::hyper::client::HttpConnector;
use google_drive3::hyper_rustls::HttpsConnector;
use google_drive3::oauth2::ServiceAccountAuthenticator;
use google_drive3::oauth2::authenticator::{HyperClientBuilder, DefaultHyperClient, Authenticator};

async fn generate_authenticator(json_path: &str) -> Authenticator<HttpsConnector<HttpConnector>> {
    let sa_key =  match oauth2::read_service_account_key(json_path).await {
        Err(e) => {
            eprintln!("error: key extraction from json: {}", e);
            exit(1)
        }
        Ok(o) => {
            println!("done: key extraction from json");
            o
        }
    };
    match ServiceAccountAuthenticator::builder(sa_key).build().await {
        Err(e) => {
            eprintln!("error: authenticator generation: {}", e);
            exit(1)
        }
        Ok(o) => {
            println!("done: authenticator generation");
            o
        }
    } 
}

async fn generate_drive_service(auth: Authenticator<HttpsConnector<HttpConnector>>) -> DriveHub<HttpsConnector<HttpConnector>>{
    println!("done: drive hub generation");
    DriveHub::new(HyperClientBuilder::build_hyper_client(DefaultHyperClient), auth)                                  
}

pub async fn generate_hub(path_to_json: &str) -> DriveHub<HttpsConnector<HttpConnector>> {
    let mut rng = rand::rngs::OsRng;
    let choice = std::fs::read_dir(path_to_json).unwrap().choose(&mut rng).unwrap().unwrap().path().to_str().unwrap().to_owned();
    println!("selected {}", choice);
    generate_drive_service(generate_authenticator(choice.as_str()).await).await
}

pub async fn create_folder(folder_name: String, parent_id: String) -> String {
    let new = File {
        name: Some(folder_name),
        parents: Some(vec![parent_id]),
        mime_type: Some("application/vnd.google-apps.folder".to_owned()),
        // parents: 
        ..Default::default()
    };
    generate_hub("sa").await
    .files().create(new)
    .supports_all_drives(true)
    .upload(std::io::empty(), "*/*".parse().unwrap())
    .await.unwrap().1.id.unwrap()
}

pub async fn copy_file(file_id: String, destination_id: String) {
    let new = File {
        parents: Some(vec![destination_id]),
        ..Default::default()                    
    };
    generate_hub("sa").await
    .files().copy(new, file_id.as_str())
    .supports_all_drives(true).doit().await.unwrap();
    println!("copied {}", file_id);
}
