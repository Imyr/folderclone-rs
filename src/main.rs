use async_recursion::async_recursion;

mod functions;

static mut TREE: Vec<DriveUnit> = Vec::new();

#[derive(Debug)]
struct DriveFile {
    name: String,
    id: String
}
#[derive(Debug)]
struct DriveFolder {
    path: String,
    id: String
}
#[derive(Debug)]
struct DriveUnit {
    key: DriveFolder,
    value: Vec<DriveFile>
}

#[async_recursion]
async fn list_files(fldr_str: String, fldr_id: String) {
    let folder = DriveFolder {
        path: (&fldr_str).to_owned(),
        id: (&fldr_id).to_owned()
    };

    let files = functions::generate_hub("sa").await.files().list()
    .supports_all_drives(true)
    .include_items_from_all_drives(true)
    .q(format!("'{}' in parents", fldr_id).as_str())
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
                    path: format!("{}/{}", fldr_str, i.name.unwrap()),
                    id: i.id.unwrap(), 
                };
                // list_files((&(entry.path)).to_owned(), (&(entry.id)).to_owned()).await;
                handles.push(tokio::spawn(list_files((&(entry.path)).to_owned(), (&(entry.id)).to_owned())));
            }}}
    let unit = DriveUnit {
        key: folder,
        value: file_list
    };
    unsafe {
        TREE.push(unit);
    }
    futures::future::join_all(handles).await;
}

async fn create_file_list(folder_id: String) {
    let name = functions::generate_hub("sa").await
    .files().get(&folder_id)
    .supports_all_drives(true).doit().await.unwrap().1.name.unwrap();
    list_files(name, folder_id).await;
}

async fn _replicate_folder_structure() {
    unsafe {
        for unit in &TREE {
            for i in unit.key.path.split("/") {
                functions::create_folder(i.to_owned(), unit.key.id.to_owned()).await;
            }
        }
    }
}

#[async_recursion]
async fn copy_files(from: String, to: String) {

    let files = functions::generate_hub("sa").await.files().list()
    .supports_all_drives(true)
    .include_items_from_all_drives(true)
    .page_size(1000)
    .q(format!("'{}' in parents", from).as_str())
    .doit().await.unwrap();

    for file in files.1.files.unwrap() {
        if file.trashed == None {
            if !(file.mime_type.unwrap() == "application/vnd.google-apps.folder")  {
                tokio::spawn(functions::copy_file(file.id.unwrap(), (&to).to_owned()));
            }
            else {
                copy_files(file.id.unwrap(), (&to).to_owned()).await;
                // tokio::spawn(list_files((&(entry.fldr_str)).to_owned(), (&(entry.fldr_id)).to_owned(), generate_hub("sa").await));
            }
        }   
    }
}

#[tokio::main]
async fn main() {
    // copy_files("1_7FBfok1ia6ZyFIg12OTSI2pC_6The83".to_string(), "1O6bG9cT1pRXKoNp-F3QoiMx3Y6F8aIPu".to_string()).await;
    _replicate_folder_structure().await;
    create_file_list("1BUea2zDuNZtL_yXe6I5Yz7LFWldPktxp".to_string()).await;
    _replicate_folder_structure().await;
    // functions::create_folder("lund md".to_owned(), "1_7FBfok1ia6ZyFIg12OTSI2pC_6The83".to_owned()).await;
    // println!("Hello, world!");
    // loop {}
}
