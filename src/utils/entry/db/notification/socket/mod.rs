use crate::utils::entry::db::notification::notify_sled_db;
use crate::utils::entry::CosmosRustServerValue;

use std::collections::HashSet;
use super::super::socket::{client_send_request, Handler, spawn_socket_service};
use std::thread::JoinHandle;
use log::info;

use serde::{Serialize,Deserialize};

pub fn spawn_socket_notification_server(socket_path: &str, tree: &sled::Db) -> JoinHandle<()> {
    info!("Spawning Unix domain socket Notification server at '{}'", socket_path);
    let task = spawn_socket_service(socket_path, Box::new(NotificationHandler{tree:tree.clone()}) as Box<dyn Handler + Send>);
    info!("Spawned Unix domain socket Notification server ready");
    task
}
pub struct NotificationHandler
{
    pub tree: sled::Db,
}
impl Handler for NotificationHandler
{
    fn process(&mut self, bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {

        let request: CosmosRustServerValue = bytes.try_into()?;

        notify_sled_db(&self.tree, request);

        let result: Vec<u8> = NotifyResult{}.try_into()?;
        Ok(result)
    }
}
pub fn client_send_notification_request(socket_path: &str, request: CosmosRustServerValue) -> anyhow::Result<NotifyResult> {
    info!("Sending notification request to Notification service at '{}'", socket_path);
    client_send_request(socket_path,request)
}

#[derive(Serialize,Deserialize,Debug)]
pub struct NotifyResult {
}

impl TryFrom<Vec<u8>> for NotifyResult {
    type Error = anyhow::Error;
    fn try_from(item: Vec<u8>) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(&item[..])?)
    }
}

impl TryFrom<NotifyResult> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(item: NotifyResult) -> anyhow::Result<Self> {
        Ok(bincode::serialize(&item)?)
    }
}
