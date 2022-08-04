// query API endpoints that are not on-chain, these are services provided by third parties or dapps.

use serde::Deserialize;
use serde::Serialize;
use enum_as_inner::EnumAsInner;

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum ServicesQuery {
    None,
    Error,
}