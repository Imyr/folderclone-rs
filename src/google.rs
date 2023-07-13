use google_drive3::api::File;
use rand::seq::IteratorRandom;
use tokio::time::{sleep, Duration};
use log::{info, warn, debug, error};
use async_recursion::async_recursion;
use google_drive3::{oauth2, DriveHub};
use google_drive3::hyper::client::HttpConnector;
use google_drive3::hyper_rustls::HttpsConnector;
use google_drive3::oauth2::ServiceAccountAuthenticator;
use google_drive3::oauth2::authenticator::{HyperClientBuilder, DefaultHyperClient};


pub async fn generate_hub(path_to_json: &str) -> DriveHub<HttpsConnector<HttpConnector>> {
    let mut rng = rand::rngs::OsRng;
    
    let dir_list = match std::fs::read_dir(path_to_json) {
        Ok(o) => o,
        Err(e) => panic!("Service accounts directory '{}': {}", crate::PATH, e),
    };
    
    let choice = dir_list.choose(&mut rng).unwrap().unwrap().path().to_str().unwrap().to_owned();
    debug!("Using SA {}", choice);
    
    let sa_key =  match oauth2::read_service_account_key(choice.as_str()).await {
        Err(e) => panic!("Key extraction from JSON: {}", e),
        Ok(o) => {
            debug!("Key extracted from JSON");
            o
        }
    };

    let auth = match ServiceAccountAuthenticator::builder(sa_key).build().await {
        Err(e) => panic!("Authenticator creation: {}", e),
        Ok(o) => {
            debug!("Authenticator generated");
            o
        }
    };
    
    debug!("Drive hub generated");
    DriveHub::new(HyperClientBuilder::build_hyper_client(DefaultHyperClient), auth)    
}

#[async_recursion]
pub async fn list_folder(parent_id: String, retries: u8) -> Option<Vec<File>> {
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
                                sleep(Duration::from_secs(crate::SLEEP)).await;
                                return list_folder(parent_id, retries-1).await
                            }
                            else {
                                error!("Folder listing '{}': {}", parent_id, e);
                                return None
                            }
                        },
                    };
                    list.append(&mut next_list);
                }
                info!("Folder listed '{}'", parent_id);
                Some(list)  
            }
            else {
                info!("Folder listed '{}'", parent_id);
                o.1.files
            }
        },
        Err(e) => {
            if retries > 0 {
                warn!("Folder listing '{}': {}", parent_id, e);
                sleep(Duration::from_secs(crate::SLEEP)).await;
                list_folder(parent_id, retries-1).await
            }
            else {
                error!("Folder listing '{}': {}", parent_id, e);
                None
            }
        },
    }
}

#[async_recursion]
pub async fn create_folder(folder_name: String, parent_id: String, retries: u8) -> Option<String> {
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
            info!("Folder created '{}'", o.1.id.clone().unwrap());
            o.1.id
        },
        Err(e) => {
            if retries > 0 {
                warn!("Folder creation in '{}': {}", parent_id, e);
                sleep(Duration::from_secs(crate::SLEEP)).await;
                create_folder(folder_name, parent_id, retries-1).await
            }
            else {
                error!("Folder creation in '{}': {}", parent_id, e);
                None
            }
        },
    }
}

#[async_recursion]
pub async fn copy_file(file_id: String, destination_id: String, retries: u8) -> Option<String> {
    let new = File {
        parents: Some(vec![destination_id.clone()]),
        ..Default::default()                    
    };
    match generate_hub(crate::PATH).await
        .files().copy(new, &file_id)
        .supports_all_drives(true).doit().await {
        Ok(o) => {
            info!("File copied '{}'", file_id);
            o.1.id
        },
        Err(e) => {
            if retries > 0 {
                warn!("File copy '{}': {}", file_id, e);
                sleep(Duration::from_secs(crate::SLEEP)).await;
                copy_file(file_id, destination_id, retries-1).await
            }
            else {
                error!("File copy '{}': {}", file_id, e);
                None
            }
        },
    }
}
