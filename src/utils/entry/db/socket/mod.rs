use anyhow::Context;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

use crate::utils::entry::CosmosRustServerValue;

pub fn encode_request(request: serde_json::Value) -> anyhow::Result<Vec<u8>> {
    Ok(request.to_string().as_bytes().to_vec())
    // &bincode::serialize(&serde_json::json!(return_msg)).unwrap()
}

pub fn get_decoded_from_stream(unix_stream: &mut UnixStream) -> anyhow::Result<serde_json::Value> {
    //let mut encoded: Vec<u8> = Vec::new();
    let mut encoded: String = "".to_string();
    unix_stream
        //.read_to_end(&mut encoded)
        .read_to_string(&mut encoded)
        .context("Failed at reading the unix stream")?;

    let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap(); // bincode::deserialize(&encoded[..]).unwrap();
    Ok(decoded)
}
pub fn get_result_decoded_from_stream(
    unix_stream: &mut UnixStream,
) -> anyhow::Result<CosmosRustServerValue> {
    let mut encoded: Vec<u8> = Vec::new();
    unix_stream
        .read_to_end(&mut encoded)
        .context("Failed at reading the unix stream")?;

    let decoded = CosmosRustServerValue::from(encoded);
    Ok(decoded)
}

pub fn client_send_request(
    socket_path: &str,
    request: serde_json::Value,
) -> anyhow::Result<CosmosRustServerValue> {
    //let socket_path = "/tmp/cosmos_rust_bot_notification_socket";
    let mut unix_stream = UnixStream::connect(socket_path).context("Could not create stream")?;

    write_request_and_shutdown(&mut unix_stream, request)?;
    read_result_from_stream(&mut unix_stream)
}

pub fn client_send_result_request(
    socket_path: &str,
    request: CosmosRustServerValue,
) -> anyhow::Result<serde_json::Value> {
    //let socket_path = "/tmp/cosmos_rust_bot_notification_socket";
    let mut unix_stream = UnixStream::connect(socket_path).context("Could not create stream")?;

    write_result_request_and_shutdown(&mut unix_stream, request)?;
    read_from_stream(&mut unix_stream)
}
fn write_result_request_and_shutdown(
    unix_stream: &mut UnixStream,
    request: CosmosRustServerValue,
) -> anyhow::Result<()> {
    unix_stream
        .write(&request.value()[..])
        .context("Failed at writing onto the unix stream")?;

    //println!("We sent a request");
    //println!("Shutting down writing on the stream, waiting for response...");

    unix_stream
        .shutdown(std::net::Shutdown::Write)
        .context("Could not shutdown writing on the stream")?;

    Ok(())
}

fn read_result_from_stream(unix_stream: &mut UnixStream) -> anyhow::Result<CosmosRustServerValue> {
    let decoded = get_result_decoded_from_stream(unix_stream)?;
    //println!("We received this response: {:?}", decoded);
    Ok(decoded)
}

fn write_request_and_shutdown(
    unix_stream: &mut UnixStream,
    request: serde_json::Value,
) -> anyhow::Result<()> {
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
    let decoded = get_decoded_from_stream(unix_stream)?;
    //println!("We received this response: {:?}", decoded);
    Ok(decoded)
}
