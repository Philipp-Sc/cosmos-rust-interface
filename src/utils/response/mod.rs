use std::collections::HashMap;
use cosmos_rust_package::api::custom::query::gov::ProposalExt;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize,Serialize};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use rust_openai_gpt_tools_socket_ipc::ipc::OpenAIGPTResult;


#[derive(Serialize,Deserialize,Debug, Clone, EnumAsInner)]
pub enum ResponseResult {
    ChainRegistry(HashMap<String,SupportedBlockchain>),
    Blockchain(BlockchainQuery),
    Services(ServicesQuery),
    SmartContracts(SmartContractsQuery),
    FraudClassification(FraudClassification),
    FraudClassificationStatus(FraudClassificationStatus),
    OpenAIGPTResult(OpenAIGPTResult),
    GPT3ResultStatus(GPT3ResultStatus),
    TaskResult(TaskResult),
    ProposalDataResult(ProposalDataResult),
    LinkToTextResult(LinkToTextResult),
    LinkToTextResultStatus(LinkToTextResultStatus),
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct TaskResult {
    pub list_of_keys_modified: Vec<String>,
}

/*
impl From<Vec<u8>> for ResponseResult {
    fn from(item: Vec<u8>) -> Self  {
        bincode::deserialize(&item[..]).unwrap()
    }
}
impl From<ResponseResult> for Vec<u8> {
    fn from(item: ResponseResult) -> Self  {
        bincode::serialize(&item).unwrap()
    }
}*/
/*
impl TryFrom<Vec<u8>> for ResponseResult {
    type Error = anyhow::Error;
    fn try_from(item: Vec<u8>) -> anyhow::Result<Self>  {
        Ok(bincode::deserialize(&item[..])?)
    }
}
impl TryFrom<ResponseResult> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(item: ResponseResult) -> anyhow::Result<Self>  {
        Ok(bincode::serialize(&item)?)
    }
}*/

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct ProposalDataResult {
    pub list_proposal_hash: Vec<u64>,
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct LinkToTextResult {
    pub link: String,
    pub text_nodes: Vec<String>,
    pub hierarchical_segmentation: Vec<Vec<bool>>,
}
impl LinkToTextResult {

    pub fn new(link: &str, text_nodes: Vec<String>, hierarchical_segmentation: Vec<Vec<bool>>, sentence_char_limit: usize) -> Self {
        let mut my_text_nodes: Vec<String> = Vec::new();
        let mut my_hierarchical_segmentation: Vec<Vec<(usize,bool)>> = hierarchical_segmentation.into_iter().map(|x| x.into_iter().map(|y| (1usize,y)).collect()).collect();
        for i in 0..text_nodes.len() {
            if text_nodes[i].chars().count() > sentence_char_limit {
                // TODO: can be improved by splitting sentences instead of whitespace. (NNSplit)
                let mut split_whitespace = text_nodes[i].split_whitespace()
                    .map(|x| format!("{} ", x))
                    .map(|split| split.chars().collect::<Vec<char>>().chunks(sentence_char_limit).map(|chunk| chunk.iter().collect::<String>()).collect::<Vec<String>>())
                    .flatten()
                    .collect::<Vec<String>>();

                let mut size_limited_paragraphs: Vec<String> = Vec::new();
                let mut paragraph = String::new();

                for word in split_whitespace {
                    if paragraph.chars().count() + word.chars().count() > sentence_char_limit {
                        size_limited_paragraphs.push(paragraph);
                        paragraph = String::new();
                    }
                    paragraph.push_str(&word);
                }
                size_limited_paragraphs.push(paragraph);

                for chunk in size_limited_paragraphs {
                    my_text_nodes.push(chunk);
                    for ii in 0..my_hierarchical_segmentation.len() {
                        my_hierarchical_segmentation[ii][i].0 = my_hierarchical_segmentation[ii][i].0 +1usize;
                    }
                }
            }else{
                my_text_nodes.push(text_nodes[i].to_owned());
            }
        }
        Self {
            link: link.to_string(),
            text_nodes: my_text_nodes,
            hierarchical_segmentation: my_hierarchical_segmentation.into_iter().map(|x| x.into_iter().map(|y| vec![y.1].repeat(y.0)).flatten().collect::<Vec<bool>>()).collect::<Vec<Vec<bool>>>(),
        }
    }
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct LinkToTextResultStatus {
    pub number_of_results: usize,
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct GPT3ResultStatus {
    pub number_of_results: usize,
}


#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct FraudClassificationStatus {
    pub number_of_classifications: usize,
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct FraudClassification { 
    pub title: String,
    pub description: String,
    pub fraud_prediction: f64,
}

#[derive(Serialize,Deserialize,Debug, Clone, EnumAsInner)]
pub enum BlockchainQuery {
    //NextKey(Option<Vec<u8>>),
    GovProposals(Vec<ProposalExt>),
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum ServicesQuery {
    None,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum SmartContractsQuery {
    None,
    Error,
}
