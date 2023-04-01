use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use cosmos_rust_package::api::core::cosmos::public_key_from_seed_phrase;
//use enum_as_inner::EnumAsInner;
//use cosmos_rust_package::api::custom::query::gov::ProposalExt;

// to query the blockchain for on-chain information
// query blocks, block height, meta information, past transactions
pub mod cosmos;



pub fn account_from_seed_phrase(seed_phrase: String, blockchain: SupportedBlockchain) -> anyhow::Result<String> {
    let pub_key = public_key_from_seed_phrase(seed_phrase)?;
    let account = pub_key.account(&blockchain.prefix)?;
    Ok(account)
}

#[cfg(test)]
mod test {

    // cargo test -- --nocapture

    use anyhow::anyhow;
    use cosmos_rust_package::api::core::cosmos::channels;

    #[cosmos_rust_package::tokio::test]
    pub async fn account_from_seed_phrase() -> anyhow::Result<()> {
        let blockchain = channels::get_supported_blockchains_from_chain_registry("./packages/chain-registry".to_string(),true,None).await.get("terra2").unwrap().clone();
        let res = super::account_from_seed_phrase("notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius".to_string(),blockchain.clone())?;
        println!("{}",&res);
        match (res.as_ref(),blockchain) {
            ("terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v", channels::SupportedBlockchain{ name , .. }) => if name=="Terra"{ Ok(())}else{Err(anyhow::anyhow!("Error"))},
            ("osmo1x46rqay4d3cssq8gxxvqz8xt6nwlz4tdyslpn7", channels::SupportedBlockchain{ name, .. }) =>  if name=="Osmosis"{ Ok(())}else{Err(anyhow::anyhow!("Error"))},
            _ => Err(anyhow::anyhow!("")),
        }
    }
}