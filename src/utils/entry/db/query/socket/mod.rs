use crate::utils::entry::{CosmosRustServerValue, Notification, UserQuery};
use std::collections::HashSet;
use super::super::socket::{client_send_request, Handler, spawn_socket_service};
use std::thread::JoinHandle;
use crate::utils::entry::db::CosmosRustBotStore;

pub fn spawn_socket_query_server(socket_path: &str, cosmos_rust_bot_store: &CosmosRustBotStore) -> JoinHandle<()> {
    println!("Starting socket query server at path: {}", socket_path);
    let task = spawn_socket_service(socket_path,Box::new(QueryHandler::new(cosmos_rust_bot_store)) as Box<dyn Handler + Send>);
    println!("Socket query server is ready and listening for incoming connections");
    task
}
pub struct QueryHandler
{
    pub cosmos_rust_bot_store: CosmosRustBotStore,
}

impl QueryHandler {
    fn new(cosmos_rust_bot_store: &CosmosRustBotStore) -> Self {
        QueryHandler{
            cosmos_rust_bot_store: cosmos_rust_bot_store.clone(),
        }
    }
}
impl Handler for QueryHandler
{
    fn process(&mut self, bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        println!("Processing user query");
        let user_query: UserQuery = UserQuery::try_from(bytes)?;
        println!("Received user query: {:?}", user_query);
        let entries = self.cosmos_rust_bot_store.handle_query(&user_query);
        let mut notification = Notification {
            query: user_query,
            entries,
            user_list: HashSet::new(),
        };
        notification.add_user_hash(notification.query.settings_part.user_hash.unwrap());
        println!("Notification created with query: {:?}", notification.query);
        let result: Vec<u8> = CosmosRustServerValue::Notification(notification).try_into()?;
        println!("Processed user query successfully");
        Ok(result)
    }
}
pub fn client_send_query_request(socket_path: &str, request: UserQuery) -> anyhow::Result<CosmosRustServerValue> {
    println!("Sending Query request to service at {}",socket_path);
    client_send_request(socket_path,request)
}