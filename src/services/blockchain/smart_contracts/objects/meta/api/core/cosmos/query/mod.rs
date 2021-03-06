use cosmos_sdk_proto::cosmos::base::query::v1beta1::PageRequest;

use cosmos_sdk_proto::cosmwasm::wasm::v1::query_client::QueryClient;
use cosmos_sdk_proto::cosmwasm::wasm::v1::*;
use cosmrs::tx::{MsgProto, Msg};
use prost_types::Any;

use cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient as AuthQueryClient;
use cosmos_sdk_proto::cosmos::auth::v1beta1::{BaseAccount, QueryAccountRequest, QueryAccountResponse};
use cosmos_sdk_proto::cosmos::vesting::v1beta1::{PeriodicVestingAccount};

use serde_json;
use std::str;

use tonic::transport::Channel;

use crate::services::blockchain::smart_contracts::objects::meta::api::core::cosmos::channels::{terra, osmosis};

use cosmos_sdk_proto::cosmos::gov::v1beta1::query_client::QueryClient as GovQueryClient;
use cosmos_sdk_proto::cosmos::gov::v1beta1::{QueryProposalRequest, QueryProposalsRequest, QueryProposalsResponse};

pub async fn get_proposals(channel: Channel, query_proposal_request: QueryProposalsRequest) -> anyhow::Result<QueryProposalsResponse> {
    /*
    StatusNil           ProposalStatus = 0x00
    StatusDepositPeriod ProposalStatus = 0x01  // Proposal is submitted. Participants can deposit on it but not vote
    StatusVotingPeriod  ProposalStatus = 0x02  // MinDeposit is reached, participants can vote
    StatusPassed        ProposalStatus = 0x03  // Proposal passed and successfully executed
    StatusRejected      ProposalStatus = 0x04  // Proposal has been rejected
    StatusFailed        ProposalStatus = 0x05  // Proposal passed but failed execution
    */
    let res = GovQueryClient::new(channel).proposals(query_proposal_request).await?.into_inner();
    Ok(res)
}



pub async fn get_contract_info(channel: Channel, address: String) -> anyhow::Result<QueryContractInfoResponse> {
    let res = QueryClient::new(channel).contract_info(QueryContractInfoRequest { address: address }).await?.into_inner();
    //println!("{:?}", &res);
    Ok(res)
}

pub async fn get_smart_contract_state<T: ?Sized + serde::Serialize>(channel: Channel, address: String, query_msg: &T) -> anyhow::Result<QuerySmartContractStateResponse> {
    let res = QueryClient::new(channel).smart_contract_state(QuerySmartContractStateRequest { address, query_data: serde_json::to_vec(query_msg)? }).await?.into_inner();
    //println!("{:?}", &res);
    Ok(res)
}

pub async fn query_account(channel: Channel, address: String) -> anyhow::Result<BaseAccount> {
    let res: QueryAccountResponse = AuthQueryClient::new(channel).account(QueryAccountRequest { address: address }).await?.into_inner();
    //println!("{:?}", res.account.as_ref().unwrap().value);
    //println!("{:?}", res.account.as_ref().unwrap().type_url);
    if let Some(account) = &res.account.as_ref() {
        if account.type_url == "/cosmos.vesting.v1beta1.PeriodicVestingAccount" {
            let periodic_vesting_account: PeriodicVestingAccount = MsgProto::from_any(&res.account.as_ref().unwrap()).unwrap();
            //println!("{:?}", periodic_vesting_account);

            let base_vesting_account = periodic_vesting_account.base_vesting_account.unwrap();
            let base_account = base_vesting_account.base_account.unwrap();
            return Ok(base_account)
        }else if account.type_url == "/cosmos.auth.v1beta1.BaseAccount" {
            let base_account: BaseAccount = MsgProto::from_any(&res.account.as_ref().unwrap()).unwrap();
            return Ok(base_account)
        }else if account.type_url == "/cosmos.auth.v1beta1.ModuleAccount" {
            return Err(anyhow::anyhow!("Error: No handler for this account type."));
        }
    }
    return Err(anyhow::anyhow!("Error"));
}


#[cfg(test)]
mod test {

    // cargo test -- --nocapture

    #[tokio::test]
    pub async fn get_proposals() -> anyhow::Result<()> {
        let channel = super::osmosis().await?;
        let res = super::get_proposals(channel,cosmos_sdk_proto::cosmos::gov::v1beta1::QueryProposalsRequest{
            proposal_status: 0x03,
            voter: "".to_string(),
            depositor: "".to_string(),
            pagination: None
        }).await?;
        Ok(())
    }


    #[tokio::test]
    pub async fn cw20_balance_via_smart_contract_state() -> anyhow::Result<()> {
        let channel = super::terra().await?;
        let query_msg = cw20::Cw20QueryMsg::Balance {
            address: "terra1vcpt3p9p6rrqaw4zwt706p8vj7uhd0sf4p5snl".to_string()
        };
        let res = super::get_smart_contract_state(channel,"terra1ecgazyd0waaj3g7l9cmy5gulhxkps2gmxu9ghducvuypjq68mq2s5lvsct".to_string(), &query_msg).await?;

        /*println!("TEST: {}", "get_smart_contract_state(address, query_msg)");
        println!("{:?}", serde_json::from_slice::<cw20::BalanceResponse>(&res.data));
        println!("{:?}", std::str::from_utf8(&res.data));*/
        Ok(())
    }

    #[tokio::test]
    pub async fn query_account() -> anyhow::Result<()> {
        let channel = super::terra().await?;
        let account = super::query_account(channel,"terra1dp0taj85ruc299rkdvzp4z5pfg6z6swaed74e6".to_string()).await?;
        /*println!("TEST: {}", "query_account(address)");
        println!("{:?}", &account);*/
        Ok(())
    }

    #[tokio::test]
    pub async fn contract_info() -> anyhow::Result<()> {
        let channel = super::terra().await?;
        let res = super::get_contract_info(channel,"terra1ccxwgew8aup6fysd7eafjzjz6hw89n40h273sgu3pl4lxrajnk5st2hvfh".to_string()).await?;
        /*println!("TEST: {}", "get_contract_info(address)");
        println!("{:?}", &res);*/
        Ok(())
    }
}