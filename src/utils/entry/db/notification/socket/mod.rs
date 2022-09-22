use crate::utils::entry::db::notification::notify_sled_db;
use crate::utils::entry::CosmosRustServerValue;
use anyhow::Context;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};

pub fn spawn_socket_notification_server(tree: &sled::Db) {
    let tree_2 = tree.clone();
    let _thread = std::thread::spawn(move || {
        loop {
            // TODO create dir with specific access rights
            let socket_path = "/tmp/cosmos_rust_bot_notification_socket";

            if std::fs::metadata(socket_path).is_ok() {
                //println!("A socket is already present. Deleting...");
                std::fs::remove_file(socket_path)
                    .with_context(|| {
                        format!("could not delete previous socket at {:?}", socket_path)
                    })
                    .unwrap();
            }

            let unix_listener = UnixListener::bind(socket_path)
                .context("Could not create the unix socket")
                .unwrap();

            loop {
                let (unix_stream, _socket_address) = unix_listener
                    .accept()
                    .context("Failed at accepting a connection on the unix listener")
                    .unwrap();
                handle_stream(unix_stream, &tree_2).unwrap();
            }
        }
    });
}

fn handle_stream(mut unix_stream: UnixStream, tree: &sled::Db) -> anyhow::Result<()> {
    let decoded = super::super::socket::get_result_decoded_from_stream(&mut unix_stream)?;
    //println!("We received this message: {:?}\nReplying...", &decoded);

    //println!("{:?}", decoded);
    notify_sled_db(tree, decoded);

    //println!("We send this response: {:?}", &field_list);

    unix_stream
        .write(&0_i32.to_ne_bytes())
        .context("Failed at writing onto the unix stream")?;

    Ok(())
}

pub fn client_send_request(request: CosmosRustServerValue) -> anyhow::Result<()> {
    let socket_path = "/tmp/cosmos_rust_bot_notification_socket";
    let _result = super::super::socket::client_send_result_request(socket_path, request)?;
    Ok(())
}
