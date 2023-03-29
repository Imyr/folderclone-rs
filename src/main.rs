use async_recursion::async_recursion;

mod functions;

use functions::{create_folder, copy_file, generate_hub};

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
    for entity in files.1.files.unwrap() {
        if entity.trashed == None {
            if !(entity.mime_type.unwrap() == "application/vnd.google-apps.folder")  {
                handles.push(tokio::spawn(copy_file(entity.id.unwrap(), (&root_id).to_string())));
            }
            else {
                handles.push(tokio::spawn(list_files(entity.name.unwrap(), entity.id.unwrap(), (&root_id).to_string())));
    }}}
    futures::future::join_all(handles).await;
}


#[tokio::main]
async fn main() {
    let folder_id_to_copy = "1nsjsxwRHgsgvzcZsQ8AucAZRUtQyQDCF";
    let destination_root_folder_id = "1A9xl_-cNkLg1E0RwXnp8H1zQGq1Uvlth";
    let source_folder_name = generate_hub("sa").await.files().get(folder_id_to_copy)
    .supports_all_drives(true)
    .doit().await.unwrap().1.name.unwrap();
    list_files(source_folder_name, folder_id_to_copy.to_string(), destination_root_folder_id.to_string()).await;
}
