use google_drive3::api::File;
use rand::seq::IteratorRandom;
use tokio::time::{sleep, Duration};
use log::{info, warn, debug, error};
use async_recursion::async_recursion;
use google_drive3::{oauth2, DriveHub};
use google_drive3::hyper::client::HttpConnector;
use google_drive3::hyper_rustls::HttpsConnector;
use google_drive3::oauth2::ServiceAccountAuthenticator;
use google_drive3::oauth2::authenticator::{HyperClientBuilder, DefaultHyperClient, Authenticator};

async fn generate_authenticator(json_path: &str) -> Authenticator<HttpsConnector<HttpConnector>> {
    let sa_key =  match oauth2::read_service_account_key(json_path).await {
        Err(e) => {
            error!("Key extraction from JSON: {}", e);
            panic!()
        }
        Ok(o) => {
            debug!("Key extracted from JSON");
            o
        }
    };
    match ServiceAccountAuthenticator::builder(sa_key).build().await {
        Err(e) => {
            error!("Authenticator creation: {}", e);
            panic!()
        }
        Ok(o) => {
            debug!("Authenticator generated");
            o
        }
    } 
}

async fn generate_drive_service(auth: Authenticator<HttpsConnector<HttpConnector>>) -> DriveHub<HttpsConnector<HttpConnector>>{
    debug!("Drive hub generated");
    DriveHub::new(HyperClientBuilder::build_hyper_client(DefaultHyperClient), auth)                                  
}

pub async fn generate_hub(path_to_json: &str) -> DriveHub<HttpsConnector<HttpConnector>> {
    let mut rng = rand::rngs::OsRng;
    let dir_list = match std::fs::read_dir(path_to_json) {
        Ok(o) => o,
        Err(e) => {
            error!("Service accounts directory '{}': {}", crate::PATH, e);
            panic!()
        },
    };
    let choice = dir_list.choose(&mut rng).unwrap().unwrap().path().to_str().unwrap().to_owned();
    debug!("Using SA {}", choice);
    generate_drive_service(generate_authenticator(choice.as_str()).await).await
}

#[async_recursion]
pub async fn list_folder(parent_id: String, retries: i8) -> Vec<File> {
    let living_hub = generate_hub(crate::PATH).await;
    match living_hub.files().list()
    .supports_all_drives(true)
    .include_items_from_all_drives(true)
    .q(format!("'{}' in parents", &parent_id).as_str())
    .page_size(1000)
    .doit().await {
        Ok(o) => {
            if !(o.1.next_page_token == None) {
                let mut list = o.1.files.unwrap();
                let mut page_token_option = o.1.next_page_token;
                while page_token_option != None {
                    let mut next_list = match living_hub.files().list()
                    .supports_all_drives(true)
                    .include_items_from_all_drives(true)
                    .q(format!("'{}' in parents", &parent_id).as_str())
                    .page_size(1000).page_token(page_token_option.unwrap().as_str())
                    .doit().await {
                        Ok(o) => {
                            if o.1.next_page_token == None {
                                page_token_option = None;
                                o.1.files.unwrap()         
                            }
                            else {
                                page_token_option = o.1.next_page_token;
                                o.1.files.unwrap()
                            }
                        },
                        Err(e) => {
                            if retries > 0 {
                                warn!("Folder listing '{}': {}", parent_id, e);
                                sleep(Duration::from_secs(4)).await;
                                return list_folder(parent_id, retries-1).await
                            }
                            else {
                                error!("Folder listing '{}': {}", parent_id, e);
                                panic!()
                            }
                        },
                    };
                    list.append(&mut next_list);
                }
                info!("Folder listed '{}'", parent_id);
                list  
            }
            else {
                info!("Folder listed '{}'", parent_id);
                o.1.files.unwrap()
            }
        },
        Err(e) => {
            if retries > 0 {
                warn!("Folder listing '{}': {}", parent_id, e);
                sleep(Duration::from_secs(4)).await;
                list_folder(parent_id, retries-1).await
            }
            else {
                error!("Folder listing '{}': {}", parent_id, e);
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
    match generate_hub(crate::PATH).await
        .files().create(new)
        .supports_all_drives(true)
        .upload(std::io::empty(), "*/*".parse().unwrap())
        .await {
        Ok(o) => {
            let id = o.1.id.unwrap();
            info!("Folder created '{}'", id);
            id
        },
        Err(e) => {
            if retries > 0 {
                warn!("Folder creation in '{}': {}", parent_id, e);
                sleep(Duration::from_secs(2)).await;
                create_folder(folder_name, parent_id, retries-1).await
            }
            else {
                error!("Folder creation in '{}': {}", parent_id, e);
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
    match generate_hub(crate::PATH).await
        .files().copy(new, &file_id)
        .supports_all_drives(true).doit().await {
        Ok(o) => {
            info!("File copied '{}'", file_id);
            o.1.id.unwrap()
        },
        Err(e) => {
            if retries > 0 {
                warn!("File copy '{}': {}", file_id, e);
                sleep(Duration::from_secs(3)).await;
                copy_file(file_id, destination_id, retries-1).await
            }
            else {
                error!("File copy '{}': {}", file_id, e);
                panic!()
            }
        },
    }
}
