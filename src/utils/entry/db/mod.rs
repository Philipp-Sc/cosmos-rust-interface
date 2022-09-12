pub mod notification;
pub mod query;
pub mod socket;

pub fn load_sled_db(path: &str) -> sled::Db {
    let db: sled::Db = sled::Config::default()
        .path(path.to_owned())
        .cache_capacity(1024 * 1024 * 1024 / 2)
        .use_compression(true)
        .compression_factor(22)
        .flush_every_ms(Some(1000))
        .open()
        .unwrap();
    db
}
