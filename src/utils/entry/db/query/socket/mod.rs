use crate::utils::entry::db::query::handle_query_sled_db;
use crate::utils::entry::{CosmosRustServerValue, Notification, UserQuery};
use std::collections::HashSet;
use super::super::socket::{client_send_request, Handler, spawn_socket_service};
use std::thread::JoinHandle;

pub fn spawn_socket_query_server(socket_path: &str, tree: &sled::Db) -> JoinHandle<()> {
    println!("spawn_socket_service startup");
    let task = spawn_socket_service(socket_path,Box::new(QueryHandler{tree:tree.clone()}) as Box<dyn Handler + Send>);
    println!("spawn_socket_service ready");
    task
}
pub struct QueryHandler
{
    pub tree: sled::Db,
}
impl Handler for QueryHandler
{
    fn process(&self, bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {

        let user_query: UserQuery = UserQuery::try_from(bytes)?;

        let entries = handle_query_sled_db(&self.tree, &user_query);
        let mut notification = Notification {
            query: user_query,
            entries,
            user_list: HashSet::new(),
        };
        notification.add_user_hash(notification.query.settings_part.user_hash.unwrap());

        let result: Vec<u8> = CosmosRustServerValue::Notification(notification).try_into()?;
        Ok(result)
    }
}
pub fn client_send_query_request(socket_path: &str, request: UserQuery) -> anyhow::Result<CosmosRustServerValue> {
    println!("client_send_request initiating");
    client_send_request(socket_path,request)
}