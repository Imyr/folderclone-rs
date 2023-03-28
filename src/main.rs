use async_recursion::async_recursion;


mod functions;

use functions::{create_folder, copy_file};

static mut TREE: Vec<DriveUnit> = Vec::new();

#[derive(Debug)]
struct DriveFile {
    name: String,
    id: String
}
#[derive(Debug)]
struct DriveFolder {
    name: String,
    id: String
}
#[derive(Debug)]
struct DriveUnit {
    key: DriveFolder,
    value: Vec<DriveFile>
}

#[async_recursion]
async fn list_files(fldr_str: String, fldr_id: String, new_root_id: String) {
    let root_id = create_folder((&fldr_str).to_owned(), new_root_id).await;

    let files = functions::generate_hub("sa").await.files().list()
    .supports_all_drives(true)
    .include_items_from_all_drives(true)
    .q(format!("'{}' in parents", fldr_id).as_str())
    .page_size(1000)
    .doit().await.unwrap();
    let mut handles = vec![];
    let mut file_list: Vec<DriveFile> = Vec::new();
    for i in files.1.files.unwrap() {
        if i.trashed == None {
            if !(i.mime_type.unwrap() == "application/vnd.google-apps.folder")  {
                let entry = DriveFile {
                    name: i.name.unwrap(),
                    id: i.id.unwrap()
                };
                file_list.push(entry);
            }
            else {
                let entry = DriveFolder {
                    name: i.name.unwrap(),
                    id: i.id.unwrap(), 
                };
                handles.push(tokio::spawn(list_files((&(entry.name)).to_owned(), (&(entry.id)).to_owned(), (&root_id).to_string())));
    }}}
    for file in file_list {
        handles.push(tokio::spawn(copy_file(file.id, (&root_id).to_string())));
    }
    futures::future::join_all(handles).await;
}


#[tokio::main]
async fn main() {
    list_files("Hynix".to_string(), "1_7FBfok1ia6ZyFIg12OTSI2pC_6The83".to_string(), "1A9xl_-cNkLg1E0RwXnp8H1zQGq1Uvlth".to_string()).await;
}
