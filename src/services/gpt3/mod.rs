use std::cmp::Ordering;
use std::collections::HashSet;
use chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, GPT3ResultStatus, TaskResult, FraudClassification, LinkToTextResult};
use rust_openai_gpt_tools_socket_ipc::ipc::{client_send_openai_gpt_embedding_request, client_send_openai_gpt_text_completion_request, OpenAIGPTResult};
use rust_openai_gpt_tools_socket_ipc::ipc::{OpenAIGPTTextCompletionResult,OpenAIGPTEmbeddingResult};
use crate::services::fraud_detection::get_key_for_fraud_detection;
use crate::services::link_to_text::{extract_links, get_key_for_link_to_text, link_to_id, string_to_hash};

use nnsplit::NNSplitOptions;
use nnsplit::tract_backend::NNSplit;
use rust_openai_gpt_tools_socket_ipc::ipc::OpenAIGPTResult::EmbeddingResult;

// note this could be done on bullet points, or summaries. as well as just the original text.
// bullet points are to inconherent, TODO: remove bullet points, and use embeddings to reduce the text.

// TODO: 1) for each text-document split into sensible list: Vec<String> , "\n" or sentence, or something smarter
// TODO: 2) generate embedding
// TODO: 3) per prompt filter text-document content list: Vec<String> by embedding ranking to the query.

// this keeps the text as it is, but removes unrelated text. this also reduces the price. as bullet points use a lot of prompts.
// embedding is 50x cheaper
// 100-200 tokens, just need smart text splitting.


// better approach idea:


// for each task:

// go over whole text, split by \n, or by sentence?!

// -> each sentence gets an embedding.
// -> sort by distance to question

// hopefully this method will censor unimportant text, and reduce the text that way.

const GPT3_PREFIX: &str = "GPT3";

const TOPIC: [&str;1] = ["Governance proposals in the Cosmos blockchain ecosystem allow stakeholders to propose and vote on changes to the protocol, including modifications to the validator set, updates to the staking and reward mechanism, and the addition or removal of features. In order to effectively communicate the intended changes and their potential impact on the network, it is important to clearly outline the problem that the proposal aims to solve and provide a detailed description of the proposed solution. It may also be helpful to present relevant data or research to support the proposal, and to consider the broader implications of the proposal on the security, scalability, and decentralization of the network. Ultimately, the success of a governance proposal depends on the ability to clearly articulate the problem and solution and to persuade the community of the value and feasibility of the proposed changes."];

const BIAS: &str = "This is an intelligent, informed and concise AI.";

const PROMPTS: [&str;3] = [
            "A string containing a brief neutral overview of the motivation or purpose behind this governance proposal (Tweet).",
            "A list of the governance proposal summarized in the form of concise bullet points (= key points,highlights,key takeaways,key ideas, noteworthy facts).",
            "The link that leads to the community discussion/forum for this proposal (if none of the links fit return None).",
               ];

const QUESTIONS: [&str;2] = [
    "Why is this proposal important? (only one sentence!)",
    "What are the potential risks or downsides? (only one sentence!)",
];

const COMMUNITY_NOTES: [&str;6] = [
    "Feasibility and technical viability (only one sentence!)",
    "Economic impact (only one sentence!)",
    "Legal and regulatory compliance (only one sentence!)",
    "Long-term sustainability (only one sentence!)",
    "Transparency & Accountability (only one sentence!)",
    "Community Support (only one sentence!)",
];

pub enum PromptKind {
    TOPIC_DESCRIPTION(usize),
    SUMMARY,
    BULLET_POINTS,
    QUESTION(usize),
    LINK_TO_COMMUNITY,
    COMMUNITY_NOTE(usize),

}

pub fn get_key_for_gpt3(hash: u64, prompt_id: &str) -> String {
    format!("{}_{}_{}",GPT3_PREFIX, prompt_id, hash)
}

pub fn get_prompt_for_gpt3(text: &str, prompt_kind: PromptKind) -> String {
    match prompt_kind {
        PromptKind::TOPIC_DESCRIPTION(index) => {
            format!("about: {}",TOPIC[index])
        }
        PromptKind::SUMMARY => {
            format!("<instruction>{}\n\n{}\n\n</instruction><source>{}</source>\n\n<result>let brief_overview: &str  = r#\"",BIAS,PROMPTS[0],text)
        },
        PromptKind::BULLET_POINTS => {
            format!("<instruction>{}\n\n{}\n\n</instruction><source>{}</source>\n\n<result>let short_hand_notes_bullet_points = [\"",BIAS,PROMPTS[1],text)
        },
        PromptKind::LINK_TO_COMMUNITY  => {
            let distance = 100;
            let mut result = String::new();
            let links = extract_links(text);

            for link in &links {
                let mut split = text.split(link);
                let before_link = split.next().unwrap_or("");
                let after_link = split.next().unwrap_or("");

                let before_start = if before_link.len() > distance {
                    before_link.len() - distance
                } else {
                    0
                };
                let after_end = if after_link.len() > distance {
                    distance
                } else {
                    after_link.len()
                };

                result.push_str(&before_link[before_start..]);
                result.push_str(link);
                result.push_str(&after_link[..after_end]);
            }

            format!("<instruction>{}\n\n</instruction><source>{}</source>\n\n<result>let maybe_selected_link: Option<String> = ",PROMPTS[2],result)
        }
        PromptKind::QUESTION(index) => {
            format!("<instruction>{}\n\nA string containing the answer to Q: {}\n\n</instruction><source>{}</source>\n\n<result max_tokens=100 max_words=75 in_one_sentence=true>let first_hand_account: &str = \"",BIAS,QUESTIONS[index],text)
        },
        PromptKind::COMMUNITY_NOTE(index) => {
            format!("<instruction>{}\n\nA string containing the {}\n\n</instruction><source>{}</source>\n\n<result max_tokens=100 max_words=75 in_one_sentence=true>let first_hand_account: &str = \"",BIAS,COMMUNITY_NOTES[index],text)
        }
    }
}

pub async fn gpt3(task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut number_of_new_results = 0usize;
    let mut number_of_stored_results = 0usize;

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {
                for each in proposals.iter_mut().filter(|x| x.status == ProposalStatus::StatusVotingPeriod) {
                    let hash = each.title_and_description_to_hash();

                    if fraud_detection_result_is_ok(&task_store,hash) {
                        let (title, description) = each.get_title_and_description();
                        let text = format!("{}/n{}", title, description);

                        let prompt = get_prompt_for_gpt3("", PromptKind::TOPIC_DESCRIPTION(0));

                        if let Ok(context) = retrieve_context_from_description_and_community_link_to_text_results_for_prompt(&task_store, &description, &prompt) {
                            error!("CONTEXT for description:\n{:?}\n\n{:?}",description,context);

                            let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", 0));
                            let prompt = get_prompt_for_gpt3(&context, PromptKind::SUMMARY);
                            let insert_result = if_key_does_not_exist_insert_openai_gpt_text_completion_result(&task_store, &key_for_hash, &prompt, 100u16);
                            insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result { Some(key_for_hash) } else { None });


                            /*
                            for i in 0..2 {
                                // ** this means this might be called more than once.
                                let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", i + 1));
                                let prompt = get_prompt_for_gpt3(&context,PromptKind::QUESTION(i));
                                let insert_result = if_key_does_not_exist_insert_openai_gpt_text_completion_result(&task_store, &key_for_hash, &prompt, 100u16);
                                insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result { Some(key_for_hash) } else { None });
                            }
                            for i in 0..6 {
                                // ** this means this might be called more than once.
                                let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", i + 1 + 2));
                                let prompt = get_prompt_for_gpt3(&context,PromptKind::COMMUNITY_NOTE(i));
                                let insert_result = if_key_does_not_exist_insert_openai_gpt_text_completion_result(&task_store, &key_for_hash, &prompt, 100u16);
                                insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result { Some(key_for_hash) } else { None });
                            }
                            */
                        }
                    }
                }
            },
            _ => {}
        }
    }
    Ok(TaskResult{
        list_of_keys_modified: keys
    })
}


pub fn insert_progress(task_store: &TaskMemoryStore, key: &str, keys: &mut Vec<String>, number_of_new_results: &mut usize, number_of_stored_results: &mut usize, insert_result: Option<String>) {
    if let Some(inserted_key) = insert_result {
        *number_of_new_results += 1usize;

        let progress: Maybe<ResponseResult> = Maybe {
            data: Ok(ResponseResult::GPT3ResultStatus(GPT3ResultStatus {
                number_of_results: *number_of_new_results + *number_of_stored_results,
            })),
            timestamp: Utc::now().timestamp(),
        };
        error!("insert_progress: {:?}",progress);

        keys.push(key.to_owned());
        task_store.push(&key, progress).ok();
    } else {
        *number_of_stored_results += 1usize;
    }
}


// TODO: fiix two bugs:
// 1) why is CONTEXT in some cases empty?
// 2) <next/> does not work correctly.


pub fn retrieve_context_from_description_and_community_link_to_text_results_for_prompt(task_store: &TaskMemoryStore, description: &str, prompt_text: &str) -> anyhow::Result<String> {


    let mut linked_text_result = retrieve_community_link_to_text_result(&task_store,description)?;

    let prompt_text_result =  LinkToTextResult{
        link: prompt_text.to_string(),
        text_nodes: vec![prompt_text.to_string()],
        hierarchical_segmentation: vec![vec![true]]
    };

    let splitter = NNSplit::load(
        "en",
        NNSplitOptions::default(),
    ).unwrap();
    let split = &splitter.split(&[description])[0];
    let sentences = split.flatten(0).iter().map(|x| x.to_string()).collect::<Vec<String>>();
    let hierarchical_segmentation = vec![sentences.iter().map(|_| true).collect::<Vec<bool>>()];

    let description_text_result = LinkToTextResult::new(description,sentences,hierarchical_segmentation,300);

    let mut linked_text = vec![description_text_result];
    match linked_text_result {
        Some(item) => {
            linked_text.push(item);
        },
        _ => {}
    };

    error!("linked_text: {:?}",linked_text);

    let mut linked_text_embeddings = Vec::new();

    let mut prompt_embedding = Vec::new();


    for chunk in &prompt_text_result.text_nodes {
        let key_for_hash = get_key_for_gpt3(string_to_hash(&chunk), "embedding");
        let mut item = if_key_does_not_exist_insert_openai_gpt_embedding_result_else_retrieve(&task_store, &key_for_hash, vec![chunk.to_string()])?;
        prompt_embedding.append(&mut item.result);
    }

    for i in 0..linked_text.len() {

            for chunk in &linked_text[i].text_nodes {

                let key_for_hash = get_key_for_gpt3(string_to_hash(&chunk), "embedding");
                let mut item = if_key_does_not_exist_insert_openai_gpt_embedding_result_else_retrieve(&task_store, &key_for_hash, vec![chunk.clone()])?;

                linked_text_embeddings.push(item.result.into_iter().zip(vec![chunk.to_string()].into_iter()).collect::<Vec<(Vec<f32>,String)>>());

            }
    }
    let linked_text_embeddings = linked_text_embeddings.into_iter().flatten().collect::<Vec<(Vec<f32>,String)>>();



    let mut linked_text_embeddings = linked_text_embeddings.into_iter().map(|x| {
        let mut sum_distance = 0f32;
        for v in 0..prompt_embedding.len() {
            let distance = cosine_similarity(&x.0, &prompt_embedding[v]);
            sum_distance = distance + sum_distance;
        }
        let average_distance = sum_distance / (prompt_embedding.len() as f32);
        (average_distance,x.1)
    }).enumerate().collect::<Vec<(usize,(f32,String))>>();


    linked_text_embeddings.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(Ordering::Equal));


    let mut my_selection = Vec::new();
    let mut chars: usize = 0;

    for i in 0..linked_text_embeddings.len(){
        let char_count = linked_text_embeddings[i].1.1.chars().count();
        if char_count + chars > 4*3500 {
            break;
        }else{
            my_selection.push(&linked_text_embeddings[i]);
            chars = char_count + chars;
        }
    }

    my_selection.sort_by(|a, b| a.0.cmp(&b.0));


    error!("my_selection: {:?}",my_selection);

    let mut result = String::new();

    for i in 0..my_selection.len(){
        result.push_str(&my_selection[i].1.1);
        if i + 1 < my_selection.len() && my_selection[i].0 + 1 !=  my_selection[i+1].0 {
            result.push_str("<next-excerpt/>");
        }
    }
    Ok(result)
}

fn cosine_similarity(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    let dot_product = vec1.iter().zip(vec2).map(|(x, y)| x * y).sum::<f32>();
    let norm1 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm2 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot_product / (norm1 * norm2)
}


pub fn retrieve_community_link_to_text_result(task_store: &TaskMemoryStore, description: &str) -> anyhow::Result<Option<LinkToTextResult>> {

    let mut extracted_links: Vec<String> = extract_links(description);

    if !extracted_links.is_empty() {

        let key_for_link_to_community = get_key_for_gpt3(string_to_hash(description), &format!("link_to_community{}", 0));
        let prompt = get_prompt_for_gpt3(description, PromptKind::LINK_TO_COMMUNITY);
        if_key_does_not_exist_insert_openai_gpt_text_completion_result(&task_store, &key_for_link_to_community, &prompt, 100u16);

        if task_store.contains_key(&key_for_link_to_community) {
            match task_store.get::<ResponseResult>(&key_for_link_to_community, &RetrievalMethod::GetOk) {
                Ok(Maybe { data: Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::TextCompletionResult(OpenAIGPTTextCompletionResult { result, .. }))), .. }) => {
                    if result.contains("None") || !result.contains("Some") {
                        extracted_links = Vec::new();
                    } else {
                        extracted_links.retain(|x| result.contains(x));
                    }
                }
                Ok(Maybe { data: Err(err), .. }) => {
                    return Err(anyhow::anyhow!(err));
                }
                Err(err) => {
                    return Err(anyhow::anyhow!(err));
                }
                _ => {
                    return Err(anyhow::anyhow!("Error: Unreachable: incorrect ResponseResult type."));
                }
            }
        }

        if !extracted_links.is_empty() {
            let link_key = get_key_for_link_to_text(&link_to_id(&extracted_links[0]));

            if task_store.contains_key(&link_key) {
                match task_store.get::<ResponseResult>(&link_key, &RetrievalMethod::GetOk) {
                    Ok(Maybe { data: Ok(ResponseResult::LinkToTextResult(link_to_text_result)), .. }) => {
                        Ok(Some(link_to_text_result))
                    }
                    Ok(Maybe { data: Err(err), .. }) => {
                        Err(anyhow::anyhow!(err))
                    }
                    Err(err) => {
                        Err(anyhow::anyhow!(err))
                    }
                    _ => {
                        Err(anyhow::anyhow!("Error: Unreachable: incorrect ResponseResult type."))
                    }
                }
            } else {
                Err(anyhow::anyhow!("Error: Unreachable: LinkToTextResult not found."))
            }
        } else {
            Ok(None)
        }
    }else{
        Ok(None)
    }

}

pub fn fraud_detection_result_is_ok(task_store: &TaskMemoryStore, hash: u64) -> bool {

    let fraud_detection_key_for_hash = get_key_for_fraud_detection(hash);

    if task_store.contains_key(&fraud_detection_key_for_hash) {
        match task_store.get::<ResponseResult>(&fraud_detection_key_for_hash, &RetrievalMethod::GetOk) {
            Ok(Maybe { data: Ok(ResponseResult::FraudClassification(FraudClassification { fraud_prediction, .. })), .. }) => {
                if fraud_prediction < 0.7 {
                    return true;
                }else {
                    return false;
                }
            }
            Err(_) => { return false; }
            _ => { return false; }
        }
    }
    return false;
}

pub fn if_key_does_not_exist_insert_openai_gpt_text_completion_result(task_store: &TaskMemoryStore, key: &str, prompt: &str, completion_token_limit: u16) -> bool {

    if !task_store.contains_key(&key) {
        
        error!("client_send_openai_gpt_text_completion_request");
        let result: anyhow::Result<OpenAIGPTResult> = client_send_openai_gpt_text_completion_request("./tmp/rust_openai_gpt_tools_socket", prompt.to_owned(), completion_token_limit);
        error!("result: {:?}",result);

        let result: Maybe<ResponseResult> = Maybe {
            data: match result {
                Ok(data) => Ok(ResponseResult::OpenAIGPTResult(data)),
                Err(err) => Err(MaybeError::AnyhowError(err.to_string())),
            },
            timestamp: Utc::now().timestamp(),
        };
        task_store.push(&key, result).ok();
        true
    }else{
        false
    }
}

pub fn if_key_does_not_exist_insert_openai_gpt_embedding_result_else_retrieve(task_store: &TaskMemoryStore, key: &str, texts: Vec<String>) -> anyhow::Result<OpenAIGPTEmbeddingResult> {

    if !task_store.contains_key(key) {
        error!("client_send_openai_gpt_embedding_request");
        let result: anyhow::Result<OpenAIGPTResult> = client_send_openai_gpt_embedding_request("./tmp/rust_openai_gpt_tools_socket", texts);
        error!("result: {:?}",result);

        let result: Maybe<ResponseResult> = Maybe {
            data: match result {
                Ok(data) => Ok(ResponseResult::OpenAIGPTResult(data)),
                Err(err) => Err(MaybeError::AnyhowError(err.to_string())),
            },
            timestamp: Utc::now().timestamp(),
        };
        task_store.push(key, result)?;
    }
    match task_store.get::<ResponseResult>(key, &RetrievalMethod::GetOk) {
        Ok(Maybe { data: Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::EmbeddingResult(embedding_result))), .. }) => {
            Ok(embedding_result)
        }
        Ok(Maybe { data: Err(err), .. }) => {
            Err(anyhow::anyhow!(err))
        }
        Err(err) => {
            Err(anyhow::anyhow!(err))
        }
        _ => {
            Err(anyhow::anyhow!("Error: Unreachable: incorrect ResponseResult type."))
        }
    }

}

