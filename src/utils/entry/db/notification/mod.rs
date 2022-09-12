use crate::utils::entry::*;

pub mod socket;

pub fn notify_sled_db(db: &sled::Db, notification: CosmosRustServerValue) {
    db.insert(notification.key(), notification.value()).ok();
}
