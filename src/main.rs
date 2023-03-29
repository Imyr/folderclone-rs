use tokio::task::JoinHandle;
use futures::future::join_all;
use clap::{Parser, Subcommand};
use async_recursion::async_recursion;

mod functions;
use functions::{create_folder, copy_file, generate_hub, list_folder};

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
async fn clone_files(fldr_str: String, fldr_id: String, new_parent_id: String) -> String {
    let parent_id = create_folder((&fldr_str).to_owned(), new_parent_id, 5).await;
    let mut handles: Vec<JoinHandle<String>> = Vec::new();
    for entity in list_folder(fldr_id, 5).await {
        if entity.trashed == None {
            if !(entity.mime_type.unwrap() == "application/vnd.google-apps.folder")  {
                handles.push(tokio::spawn(copy_file(entity.id.unwrap(), (&parent_id).to_string(), 3)));
            }
            else {
                handles.push(tokio::spawn(clone_files(entity.name.unwrap(), entity.id.unwrap(), (&parent_id).to_string())));
    }}}
    join_all(handles).await;
    parent_id
}


#[tokio::main]
async fn main() {
    match Cli::parse().command {
        Commands::Copy {source, destination} => {
            let source_folder_name = match generate_hub("sa").await.files().get(source.as_str())
                                            .supports_all_drives(true).doit().await {
                    Ok(o) => o.1.name.unwrap(),
                    Err(e) => {
                        eprintln!("Failure: Folder Name Retrieval of '{}': {}", source.as_str(), e);
                        "Clone".to_string()
                    },
                };
            let parent_id = clone_files(source_folder_name.to_owned(), source.to_owned(), destination.to_owned()).await;            
            println!("Copied {} ({}) to {}.", source_folder_name, source, parent_id);
        }
    };
}
