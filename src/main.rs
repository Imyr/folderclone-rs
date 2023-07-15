use log::{info, error};
use tokio::task::JoinHandle;
use futures::future::join_all;
use clap::{Parser, Subcommand};
use async_recursion::async_recursion;

pub const PATH: &str = "accounts";
pub const RETRIES: u8 = 2;
pub const SLEEP: u64 = 3;

mod google;
use google::{create_folder, copy_file, generate_hub, list_folder};

#[derive(Subcommand)]
enum Commands {
    /// Copy from Source ID to inside Destination ID
    Copy {
        source: String, 
        destination: String,
    },
}

/// Clone utility developed by House of the Alchemist
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[async_recursion]
async fn copy(fldr_str: String, fldr_id: String, new_parent_id: String) -> Option<String> {
    let parent_id = create_folder((&fldr_str).to_owned(), new_parent_id, RETRIES).await.unwrap();
    let mut handles: Vec<JoinHandle<Option<String>>> = Vec::new();
    let folders = list_folder(fldr_id, RETRIES).await;
    if folders.is_none() {
        return Some(parent_id)
    }
    for entity in folders.unwrap() {
        if entity.trashed == None {
            if !(entity.mime_type.unwrap() == "application/vnd.google-apps.folder")  {
                handles.push(tokio::spawn(copy_file(entity.id.unwrap(), (&parent_id).to_string(), RETRIES)));
            }
            else {
                handles.push(tokio::spawn(copy(entity.name.unwrap(), entity.id.unwrap(), (&parent_id).to_string())));
    }}}
    join_all(handles).await;
    Some(parent_id)
}


#[tokio::main]
async fn main() {
    env_logger::init();
    match Cli::parse().command {
        Commands::Copy {source, destination} => {
            match generate_hub(PATH).await.files().get(source.as_str())
                                    .supports_all_drives(true).doit().await {
                Ok(o) => {
                let source_name = o.1.name.unwrap();
                    if o.1.mime_type.unwrap() == "application/vnd.google-apps.folder" {
                        info!("Copying {} ({}) to {}", source_name, source, destination);
                        let id = copy(source_name.to_owned(), source.to_owned(), destination.to_owned()).await;            
                        info!("Copied {} ({}) to {}.", source_name, source, id.unwrap());
                    } else {
                        info!("Copying {} ({}) to {}", source_name, source, destination);
                        let id = copy_file(source.to_owned(), destination.to_owned(), RETRIES).await;
                        info!("Copied {} ({}) to {}.", source_name, source, id.unwrap());
                    }
                }
                Err(e) => {
                    error!("Folder name retrieval of '{}': {}", source, e);
                },
            };       
        }
    };
}
