
use crate::services::blockchain::smart_contracts::objects::meta::api::core::*;
use crate::services::blockchain::smart_contracts::objects::ResponseResult;

// outputs always an json! type

pub async fn get_proposals(blockchain: &str) -> anyhow::Result<ResponseResult> {
    let channel = cosmos::channels::channel(blockchain).await?;
    let res = cosmos::query::get_proposals(channel, cosmos_sdk_proto::cosmos::gov::v1beta1::QueryProposalsRequest {
        proposal_status: 0x03,
        voter: "".to_string(),
        depositor: "".to_string(),
        pagination: None
    }).await?;
    let mut list = Vec::new();
    for proposal in res.proposals {
        let p = &proposal.content.unwrap();
        if p.type_url == "/cosmos.gov.v1beta1.TextProposal" {
            let t: cosmos_sdk_proto::cosmos::gov::v1beta1::TextProposal = cosmrs::tx::MsgProto::from_any(p).unwrap();
            list.push(format!("{:?}",t));
            //println!("{:?}", t);
        } else if p.type_url == "/cosmos.distribution.v1beta1.CommunityPoolSpendProposal" {
            let t: cosmos_sdk_proto::cosmos::distribution::v1beta1::CommunityPoolSpendProposal = cosmrs::tx::MsgProto::from_any(p).unwrap();
            list.push(format!("{:?}",t));
            //println!("{:?}", t);
        } else if p.type_url == "/cosmos.params.v1beta1.ParameterChangeProposal" {
            let t: cosmos_sdk_proto::cosmos::params::v1beta1::ParameterChangeProposal = cosmrs::tx::MsgProto::from_any(p).unwrap();
            list.push(format!("{:?}",t));
            //println!("{:?}", t);
        } else if p.type_url == "/cosmos.upgrade.v1beta1.SoftwareUpgradeProposal" {
            let t: cosmos_sdk_proto::cosmos::upgrade::v1beta1::SoftwareUpgradeProposal = cosmrs::tx::MsgProto::from_any(p).unwrap();
            list.push(format!("{:?}",t));
            //println!("{:?}", t);
        } else if p.type_url == "/ibc.core.client.v1.ClientUpdateProposal" {
            let t: cosmos_sdk_proto::ibc::core::client::v1::ClientUpdateProposal = cosmrs::tx::MsgProto::from_any(p).unwrap();
            list.push(format!("{:?}",t));
            //println!("{:?}", t);
        } else if p.type_url == "/osmosis.poolincentives.v1beta1.UpdatePoolIncentivesProposal" {
            let t: osmosis_proto::osmosis::poolincentives::v1beta1::UpdatePoolIncentivesProposal = osmosis_proto::custom_cosmrs::MsgProto::from_any(p).unwrap();
            list.push(format!("{:?}",t));
            //println!("{:?}", t);
        } else {
            //println!("{:?}", p);
        }
    }
    Ok(ResponseResult::Json(serde_json::json!(list)))
}


#[cfg(test)]
mod test {

    // cargo test -- --nocapture

    #[tokio::test]
    pub async fn get_proposals() -> anyhow::Result<()> {
        let res = super::get_proposals("terra").await?;
        println!("{:?}",res);
        Ok(())
    }
}