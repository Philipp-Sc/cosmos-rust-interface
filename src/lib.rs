pub mod blockchain;
pub mod services;
pub mod smart_contracts;

pub mod utils;

use blockchain::BlockchainQuery;
use services::ServicesQuery;
use smart_contracts::SmartContractsQuery;
use enum_as_inner::EnumAsInner;


#[derive(Debug, Clone, EnumAsInner)]
pub enum ResponseResult {
    Blockchain(BlockchainQuery),
    Services(ServicesQuery),
    SmartContracts(SmartContractsQuery),
    LogEntry(String),  // used for logging
    Text(String),  // used for logs
}


#[cfg(test)]
mod test {

    // cargo test -- --nocapture

    use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
    use cosmos_rust_package::api::custom::query::gov::ProposalStatus;

    #[tokio::test]
    pub async fn get_proposals() -> anyhow::Result<()> {
        let res = super::blockchain::cosmos::gov::get_proposals(SupportedBlockchain::Terra, ProposalStatus::StatusPassed).await?;
        println!("{:#?}",res.as_blockchain().unwrap().as_gov_proposals().unwrap()[0].content());
        Ok(())
    }
}