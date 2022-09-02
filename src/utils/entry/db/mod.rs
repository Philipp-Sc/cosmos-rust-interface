use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use anyhow::Context;
use serde::Serialize;
use crate::utils::entry::db::query::query_sled_db;

pub mod query;


pub fn load_sled_db(path: &str) -> sled::Db {
    let db: sled::Db = sled::Config::default()
        .path(path.to_owned())
        .cache_capacity(1024 * 1024 * 1024 / 2)
        .use_compression(true)
        .compression_factor(22)
        .flush_every_ms(Some(1000)).open().unwrap();
    db
}

pub fn spawn_socket_query_server(tree: &sled::Db) {
    let tree_2 = tree.clone();
    let thread = std::thread::spawn(move || {
        loop {
            let socket_path = "/tmp/cosmos_rust_bot_socket";

            if std::fs::metadata(socket_path).is_ok() {
                //println!("A socket is already present. Deleting...");
                std::fs::remove_file(socket_path).with_context(|| {
                    format!("could not delete previous socket at {:?}", socket_path)
                }).unwrap();
            }

            let unix_listener =
                UnixListener::bind(socket_path).context("Could not create the unix socket").unwrap();

            loop {
                let (unix_stream, _socket_address) = unix_listener
                    .accept()
                    .context("Failed at accepting a connection on the unix listener").unwrap();
                handle_stream(unix_stream,&tree_2).unwrap();
            }
        }
    });
}

fn handle_stream(mut unix_stream: UnixStream,tree: &sled::Db) -> anyhow::Result<()> {
    let decoded= get_decoded_from_stream(&mut unix_stream)?;
    //println!("We received this message: {:?}\nReplying...", &decoded);


    let empty: Vec<serde_json::Value> = Vec::new();
    let fields = decoded.get("fields").map(|x| x.as_array().unwrap_or(&empty)).unwrap_or(&empty).iter().map(|x| x.as_str().unwrap_or("").to_string()).collect::<Vec<String>>();

    let subset = query_sled_db(tree, decoded);
    let mut field_list:Vec<serde_json::Value> = Vec::new();


    for i in 0..subset.len() {
        let mut m = serde_json::json!({});
        for field in fields.iter() {
            if let Some(val) = subset[i].try_get(field) {
                if let Some(summary_text) = val.as_str() {
                    m.as_object_mut().unwrap().insert(field.to_string(),serde_json::json!(summary_text.to_string()));
                }
            }
        }
        field_list.push(m);
    }


    //println!("We send this response: {:?}", &field_list);

    unix_stream
        .write(&encode_request(serde_json::json!(field_list))?[..])
        .context("Failed at writing onto the unix stream")?;

    Ok(())
}

fn encode_request(request: serde_json::Value) -> anyhow::Result<Vec<u8>> {
    Ok(request.to_string().as_bytes().to_vec())

    // &bincode::serialize(&serde_json::json!(return_msg)).unwrap()
    }

fn get_decoded_from_stream(unix_stream: &mut UnixStream) -> anyhow::Result<serde_json::Value> {
    //let mut encoded: Vec<u8> = Vec::new();
    let mut encoded: String = "".to_string();
    unix_stream
        //.read_to_end(&mut encoded)
        .read_to_string(&mut encoded)
        .context("Failed at reading the unix stream")?;

    let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap(); // bincode::deserialize(&encoded[..]).unwrap();
    Ok(decoded)
}

pub fn client_send_request(request: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let socket_path = "/tmp/cosmos_rust_bot_socket";

    let mut unix_stream =
        UnixStream::connect(socket_path).context("Could not create stream")?;

    write_request_and_shutdown(&mut unix_stream,request)?;
    read_from_stream(&mut unix_stream)
}
fn write_request_and_shutdown(unix_stream: &mut UnixStream,request: serde_json::Value) -> anyhow::Result<()> {
    unix_stream
        .write(&encode_request(request)?[..])
        .context("Failed at writing onto the unix stream")?;

    //println!("We sent a request");
    //println!("Shutting down writing on the stream, waiting for response...");

    unix_stream
        .shutdown(std::net::Shutdown::Write)
        .context("Could not shutdown writing on the stream")?;

    Ok(())
}

fn read_from_stream(unix_stream: &mut UnixStream) -> anyhow::Result<serde_json::Value> {
    let decoded= get_decoded_from_stream(unix_stream)?;
    //println!("We received this response: {:?}", decoded);
    Ok(decoded)
}