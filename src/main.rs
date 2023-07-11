use log::{info, error};
use tokio::task::JoinHandle;
use futures::future::join_all;
use clap::{Parser, Subcommand};
use async_recursion::async_recursion;

pub const PATH: &str = "accounts";

mod google;
use google::{create_folder, copy_file, generate_hub, list_folder};

#[derive(Subcommand)]
enum Commands {
    /// Copy from Source ID to inside Destination ID
    Copy {source: String, destination: String},
}

/// Clone utility developed by House of the Alchemist
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[async_recursion]
async fn copy(fldr_str: String, fldr_id: String, new_parent_id: String) -> String {
    let parent_id = create_folder((&fldr_str).to_owned(), new_parent_id, 5).await;
    let mut handles: Vec<JoinHandle<String>> = Vec::new();
    for entity in list_folder(fldr_id, 5).await {
        if entity.trashed == None {
            if !(entity.mime_type.unwrap() == "application/vnd.google-apps.folder")  {
                handles.push(tokio::spawn(copy_file(entity.id.unwrap(), (&parent_id).to_string(), 3)));
            }
            else {
                handles.push(tokio::spawn(copy(entity.name.unwrap(), entity.id.unwrap(), (&parent_id).to_string())));
    }}}
    join_all(handles).await;
    parent_id
}


#[tokio::main]
async fn main() {
    env_logger::init();
    match Cli::parse().command {
        Commands::Copy {source, destination} => {
            match generate_hub(PATH).await.files().get(source.as_str())
                                    .supports_all_drives(true).doit().await {
                Ok(o) => {
                    let source_folder_name = o.1.name.unwrap();
                    info!("{}", format!("Copying {} ({}) to {}", source_folder_name, source, destination));
                    let parent_id = copy(source_folder_name.to_owned(), source.to_owned(), destination.to_owned()).await;            
                    info!("Copied {} ({}) to {}.", source_folder_name, source, parent_id);
                }
                Err(e) => {
                    error!("Folder name retrieval of '{}': {}", source, e);
                },
            };       
        }
    };
}
