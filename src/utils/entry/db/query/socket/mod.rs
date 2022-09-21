use crate::utils::entry::db::query::handle_query_sled_db;
use crate::utils::entry::{CosmosRustServerValue, Notification};
use anyhow::Context;
use std::collections::HashSet;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};

pub fn spawn_socket_query_server(tree: &sled::Db) {
    let tree_2 = tree.clone();
    let _thread = std::thread::spawn(move || {
        loop {
            let socket_path = "/tmp/cosmos_rust_bot_query_socket";

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
    let decoded = super::super::socket::get_decoded_from_stream(&mut unix_stream)?;
    //println!("We received this message: {:?}\nReplying...", &decoded);

    let user_hash = &decoded
        .get("user_hash")
        .map(|x| x.as_u64().unwrap_or(0))
        .unwrap_or(0);
    let mut notification = Notification {
        query: decoded.to_string(),
        entries: handle_query_sled_db(tree, decoded.clone()),
        user_list: HashSet::new(),
    };
    notification.add_user_hash(*user_hash);

    //println!("We send this response: {:?}", &field_list);

    unix_stream
        .write(&CosmosRustServerValue::Notification(notification).value()[..])
        .context("Failed at writing onto the unix stream")?;

    Ok(())
}

pub fn client_send_request(request: serde_json::Value) -> anyhow::Result<CosmosRustServerValue> {
    let socket_path = "/tmp/cosmos_rust_bot_query_socket";
    super::super::socket::client_send_request(socket_path, request)
}
