pub mod blockchain;
pub mod services;
pub mod smart_contracts;

use blockchain::BlockchainQuery;
use services::ServicesQuery;
use smart_contracts::SmartContractsQuery;
use enum_as_inner::EnumAsInner;


#[derive(Debug, Clone, EnumAsInner)]
pub enum ResponseResult {
    Blockchain(BlockchainQuery),
    Services(ServicesQuery),
    SmartContracts(SmartContractsQuery),
    Text(String),
}


#[cfg(test)]
mod test {

    // cargo test -- --nocapture

    #[tokio::test]
    pub async fn governance() -> anyhow::Result<()> {
        let res = super::blockchain::cosmos::gov::governance().await?;
        let response_result = super::ResponseResult::Blockchain(res);
        println!("{:#?}",response_result.as_blockchain().unwrap().as_gov_proposals().unwrap()[0].content());
        Ok(())
    }
}