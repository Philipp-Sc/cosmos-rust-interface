use crate::utils::entry::db::notification::notify_sled_db;
use crate::utils::entry::CosmosRustServerValue;

use std::collections::HashSet;
use super::super::socket::{client_send_request, Handler, spawn_socket_service};
use std::thread::JoinHandle;

pub fn spawn_socket_notification_server(socket_path: &str, tree: &sled::Db) -> JoinHandle<()> {
    println!("spawn_socket_service startup");
    let task = spawn_socket_service(socket_path,Box::new(NotificationHandler{tree:tree.clone()}) as Box<dyn Handler + Send>);
    println!("spawn_socket_service ready");
    task
}
pub struct NotificationHandler
{
    pub tree: sled::Db,
}
impl Handler for NotificationHandler
{
    fn process(&self, bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {

        let request: CosmosRustServerValue = bytes.try_into()?;

        notify_sled_db(&self.tree, request);

        let result: Vec<u8> = NotifyResult{}.try_into()?;
        Ok(result)
    }
}
pub fn client_send_notification_request(socket_path: &str, request: CosmosRustServerValue) -> anyhow::Result<NotifyResult> {
    println!("client_send_request initiating");
    client_send_request(socket_path,request)
}

#[derive(serde::Serialize,serde::Deserialize,Debug)]
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
