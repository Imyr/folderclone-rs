use google_drive3::api::File;
use rand::seq::IteratorRandom;
use async_recursion::async_recursion;
use tokio::time::{sleep, Duration};
use google_drive3::{oauth2, DriveHub};
use google_drive3::hyper::client::HttpConnector;
use google_drive3::hyper_rustls::HttpsConnector;
use google_drive3::oauth2::ServiceAccountAuthenticator;
use google_drive3::oauth2::authenticator::{HyperClientBuilder, DefaultHyperClient, Authenticator};

async fn generate_authenticator(json_path: &str) -> Authenticator<HttpsConnector<HttpConnector>> {
    let sa_key =  match oauth2::read_service_account_key(json_path).await {
        Err(e) => {
            eprintln!("Failure: 'Key Extraction from JSON': {}", e);
            panic!()
        }
        Ok(o) => {
            // println!("done: key extraction from json");
            o
        }
    };
    match ServiceAccountAuthenticator::builder(sa_key).build().await {
        Err(e) => {
            eprintln!("Failure: 'Authenticator Generation': {}", e);
            panic!()
        }
        Ok(o) => {
            // println!("done: authenticator generation");
            o
        }
    } 
}

async fn generate_drive_service(auth: Authenticator<HttpsConnector<HttpConnector>>) -> DriveHub<HttpsConnector<HttpConnector>>{
    // println!("done: drive hub generation");
    DriveHub::new(HyperClientBuilder::build_hyper_client(DefaultHyperClient), auth)                                  
}

pub async fn generate_hub(path_to_json: &str) -> DriveHub<HttpsConnector<HttpConnector>> {
    let mut rng = rand::rngs::OsRng;
    let choice = std::fs::read_dir(path_to_json).unwrap().choose(&mut rng).unwrap().unwrap().path().to_str().unwrap().to_owned();
    // println!("selected {}", choice);
    generate_drive_service(generate_authenticator(choice.as_str()).await).await
}

#[async_recursion]
pub async fn list_folder(parent_id: String, retries: i8) -> Vec<File> {
    match generate_hub("sa").await.files().list()
    .supports_all_drives(true)
    .include_items_from_all_drives(true)
    .q(format!("'{}' in parents", parent_id).as_str())
    .page_size(1000)
    .doit().await {
        Ok(o) => o.1.files.unwrap(),
        Err(e) => {
            if retries > 0 {
                eprintln!("Retrying: 'Listing Files' in '{}': Waiting for 4 seconds: {} tries left", parent_id, retries-1);
                sleep(Duration::from_secs(4)).await;
                list_folder(parent_id, retries-1).await
            }
            else {
                eprintln!("Failure: 'Listing Files' in '{}': {}", parent_id, e);
                panic!()
            }
        },
    }
}

#[async_recursion]
pub async fn create_folder(folder_name: String, parent_id: String, retries: i8) -> String {
    let new = File {
        name: Some(folder_name.clone()),
        parents: Some(vec![parent_id.clone()]),
        mime_type: Some("application/vnd.google-apps.folder".to_owned()),
        ..Default::default()
    };
    match generate_hub("sa").await
        .files().create(new)
        .supports_all_drives(true)
        .upload(std::io::empty(), "*/*".parse().unwrap())
        .await {
        Ok(o) => o.1.id.unwrap(),
        Err(e) => {
            if retries > 0 {
                eprintln!("Retrying: 'New Folder Creation' in '{}': Waiting for 2 seconds: {} tries left", parent_id, retries-1);
                sleep(Duration::from_secs(2)).await;
                create_folder(folder_name, parent_id, retries-1).await
            }
            else {
                eprint!("Failure: 'New Folder Creation' in '{}': {}", parent_id, e);
                panic!()
            }
        },
    }
}

#[async_recursion]
pub async fn copy_file(file_id: String, destination_id: String, retries: i8) -> String {
    let new = File {
        parents: Some(vec![destination_id.clone()]),
        ..Default::default()                    
    };
    match generate_hub("sa").await
        .files().copy(new, &file_id)
        .supports_all_drives(true).doit().await {
        Ok(o) => o.1.id.unwrap(),
        Err(e) => {
            if retries > 0 {
                eprintln!("Retrying: File Copy '{}': Waiting for 3 seconds: {} tries left", file_id, retries-1);
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                copy_file(file_id, destination_id, retries-1).await
            }
            else {
                eprint!("Failure: File Copy '{}': {}", file_id, e);
                panic!()
            }
        },
    }
    // println!("Done: File Copy '{}'", file_id);
}
