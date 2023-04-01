//#![feature(return_position_impl_trait_in_trait)]

pub use cosmos_rust_package;
//pub use sled;
//pub use bincode;

#[cfg(feature = "interface")]
pub mod blockchain;
#[cfg(feature = "interface")]
pub mod services;
#[cfg(feature = "interface")]
pub mod smart_contracts;

pub mod utils;


#[cfg(test)]
mod test {

    // cargo test -- --nocapture

    use cosmos_rust_package::api::core::cosmos::channels;
    use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
    use cosmos_rust_package::api::custom::query::gov::ProposalStatus;

    #[cosmos_rust_package::tokio::test]
    pub async fn get_proposals() -> anyhow::Result<()> {
        let res = super::blockchain::cosmos::gov::get_proposals(channels::get_supported_blockchains_from_chain_registry("./packages/chain-registry".to_string(),true,None).await.get("terra2").unwrap().clone(), ProposalStatus::StatusPassed).await?;
        println!("{:#?}",res.as_blockchain().unwrap().as_gov_proposals().unwrap()[0].blockchain_name);
        println!("{:#?}",res.as_blockchain().unwrap().as_gov_proposals().unwrap()[0].status);
        println!("{:#?}",res.as_blockchain().unwrap().as_gov_proposals().unwrap()[0].content);
        Ok(())
    }
}