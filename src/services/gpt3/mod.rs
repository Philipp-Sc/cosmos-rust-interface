use std::cmp::Ordering;
use std::collections::HashSet;
use anyhow::anyhow;
use cosmos_rust_package::chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::types::gov::proposal_ext::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, GPT3ResultStatus, TaskResult, FraudClassification, LinkToTextResult};
use rust_openai_gpt_tools_socket_ipc::ipc::{client_send_openai_gpt_chat_completion_request, client_send_openai_gpt_embedding_request, client_send_openai_gpt_text_completion_request, OpenAIGPTResult};
use rust_openai_gpt_tools_socket_ipc::ipc::{OpenAIGPTChatCompletionResult,OpenAIGPTEmbeddingResult};
use crate::services::fraud_detection::{get_key_for_fraud_detection, validate_fraud_detection_result};
use crate::services::link_to_text::{extract_links, get_key_for_link_to_text, link_to_id, string_to_hash, retrieve_embedded_data_from_links, get_embedded_data_from_link};

use nnsplit::NNSplitOptions;
use nnsplit::tract_backend::NNSplit;
use rust_openai_gpt_tools_socket_ipc::ipc::OpenAIGPTResult::EmbeddingResult;
use crate::blockchain::cosmos::gov::get_key_for_tally_result;

const GPT3_PREFIX: &str = "GPT3";

const GPT_4_8K_MODEL: &str ="gpt-4";
const GPT_4_32K_MODEL: &str ="gpt-4-32k";
const GPT_3_5_TURBO_MODEL: &str = "gpt-3.5-turbo";

const SYSTEM_RETRIEVE_OPTION: &str = r#"
As the Rust compiler,
check if a specified string exists in the given source code.
Response type should be Option<&str>.
Follow the guideline to approach the task step by step,
and ensure that you thoroughly analyze the source code to determine if the specified string is present.
Response should be Some(&str) with the string value if found, or None if not found."#;


const SYSTEM_SUMMARY: &str = r#"
You are tasked with generating truthful and reliable content.
Your responses should adhere to journalistic integrity and ethical principles.
Avoid making unfounded assumptions and do not show links/URLs.
JimBrowning, Coffeezilla, Seymour Hersh, Ida B. Wells, Glenn Greenwald and Brian Krebs are examples of reputable sources in the field of investigative journalism.
Think critically, use first principles, and approach each task step by step to ensure accurate and contextually appropriate output.
Fraudulent content is not tolerated and should be flagged for review without explanation.
Your role is to provide reliable information to readers.
Generate content that upholds the highest standards of journalism."#;

const SYSTEM_QUESTIONS: &str = r#"
You go over and beyond to assist the user.
You are tasked with generating truthful and reliable content.
Your responses should adhere to journalistic integrity and ethical principles.
Avoid making unfounded assumptions and do not show links/URLs.
JimBrowning, Coffeezilla, Seymour Hersh, Ida B. Wells, Glenn Greenwald and Brian Krebs are examples of reputable sources in the field of investigative journalism.
Think critically, use first principles, and approach each task step by step to ensure accurate and contextually appropriate output.
Fraudulent content is not tolerated and should be flagged for review without explanation.
Your role is to provide reliable information to readers.
Generate content that upholds the highest standards of journalism.""#;

const TOPICS_FOR_EMBEDDING: [&str;3] = [
    "Governance proposals in the Cosmos blockchain ecosystem allow stakeholders to propose and vote on changes to the protocol, including modifications to the validator set, updates to the staking and reward mechanism, and the addition or removal of features. In order to effectively communicate the intended changes and their potential impact on the network, it is important to clearly outline the problem that the proposal aims to solve and provide a detailed description of the proposed solution. It may also be helpful to present relevant data or research to support the proposal, and to consider the broader implications of the proposal on the security, scalability, and decentralization of the network. Ultimately, the success of a governance proposal depends on the ability to clearly articulate the problem and solution and to persuade the community of the value and feasibility of the proposed changes.",
    "Community notes are a way for people to collaborate and provide additional context or information on a proposal. By allowing a diverse group of contributors to leave notes and rate the helpfulness of those notes, the goal is to create a more informed and balanced understanding of the proposal. This can help to ensure that decision-making processes are based on accurate and complete information. There are a few key considerations to keep in mind when using community notes: Encourage diverse perspectives: It's important to encourage contributors from different backgrounds and viewpoints to leave notes. This can help to provide a more balanced and comprehensive understanding of the proposal. Verify information: It's important to verify the accuracy of any information provided in a community note. This can help to ensure that the notes are reliable and helpful to others.     Be respectful and civil: It's important to maintain a respectful and civil tone when leaving a community note. Personal attacks or inflammatory language are not productive and can discourage others from contributing. Overall, community notes can be a valuable tool for fostering collaboration and improving the quality of information available when considering a proposal.",
    "A great summary: Concise: It should be brief and to the point, providing the most important information without going into unnecessary detail. Accurate: It should accurately convey the main points and key arguments of the original material, without distorting or misinterpreting the information. Comprehensive: It should cover all of the major points and key arguments of the original material, providing a complete and thorough understanding of the topic. Neutral: It should present the information objectively, without introducing personal bias or opinion. Well-organized: It should be organized in a logical and coherent manner, making it easy to understand and follow. Clear: It should use language that is easy to understand, avoiding jargon or technical terms that may be confusing to readers. By following these principles, a great summary can effectively condense complex information and present it in a way that is easy to understand and comprehend."
];

const PROMPTS: [&str;3] = [
            "Provide a quick overview of this governance proposal.",
            "Extract the link leading to the community discussion/forum for this proposal.",
            "Generate a briefing in which the following questions/points are answered if applicable/relevant:"
               ];

const QUESTIONS: [&str;8] = [
    "Why is this proposal important? (in the style of a summary, title: ❓ Problem)",
    "What are the potential risks or downsides? (in the style of a reasonable warning, title: ⚠ Risks)",
    "Is this proposal feasible and viable? (in the style of a correct technical assessment, title: 🛠 Feasibility/viability)",
    "What is the economic impact? (in the style of a balanced economic analysis, title: 💸 Economic impact)",
    "Is it legally compliant? (in the style of a fair legal review, title: ⚖ Legal compliance)",
    "Is it sustainable? (in the style of an environmental assessment, title: 🌿 Sustainability)",
    "Is it transparent and accountable? (in the style of a truthful transparency report, title: 🔎 Transparency/accountability)",
    "Is there community support? (in the style of a social feedback assessment, title: 👥 Community Support)"
];


pub enum PromptKind {
    SUMMARY,
    QUESTIONS,
    LINK_TO_COMMUNITY,
}

pub fn get_key_for_gpt3(hash: u64, prompt_id: &str) -> String {
    format!("{}_{}_{}",GPT3_PREFIX, prompt_id, hash)
}

pub fn get_prompt_for_gpt3(text: &str, prompt_kind: PromptKind) -> String {
    match prompt_kind {
        PromptKind::SUMMARY => {
            format!("<instruction>{}</instruction><source>{}</source><result>",PROMPTS[0],text)
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

            format!("<instruction>{}</instruction><source>{}</source><result>",PROMPTS[1],result)
        }
        PromptKind::QUESTIONS => {
            let mut questions = String::new();
            questions.push_str("\n");
            for each in QUESTIONS {
                questions.push_str("\n- ");
                questions.push_str(each);
            }
            format!("<instruction>{}{}</instruction><source>{}</source><result>",PROMPTS[2],questions,text)
        },
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
                    let hash = each.object_to_hash();

                    if validate_fraud_detection_result(&task_store,hash) {

                        let title = each.get_title();
                        let description = each.get_description();
                        let text = format!("{}/n{}", title, description);

                        match retrieve_context_from_description_and_community_link_to_text_results_for_prompt(&task_store, &description, TOPICS_FOR_EMBEDDING.iter().map(|&s| s.to_string()).collect()) {
                            Ok(context) => {
                                debug!("Successfully retrieved context for prompt. (hash: {})",hash);
                                debug!("Context:\n{:?}", context);

                                // SUMMARY
                                let key_for_hash = get_key_for_gpt3(hash, &format!("SUMMARY_{}", 0));
                                let prompt = get_prompt_for_gpt3(&context, PromptKind::SUMMARY);
                                try_get_or_insert_chat_completion_result(&task_store, &key_for_hash, &GPT_4_8K_MODEL, &SYSTEM_SUMMARY, &prompt, 200u16).ok();


                                // BRIEFING
                                let key_for_hash = get_key_for_gpt3(hash, &format!("BRIEFING_{}", 0));
                                let prompt = get_prompt_for_gpt3(&context, PromptKind::QUESTIONS);
                                try_get_or_insert_chat_completion_result(&task_store, &key_for_hash, &GPT_4_8K_MODEL, &SYSTEM_QUESTIONS, &prompt, 800u16).ok();

                            }
                            Err(err) => {
                                error!("Failed to retrieve context for prompt: {}", err.to_string());
                            }
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

/*
pub fn insert_progress(task_store: &TaskMemoryStore, key: &str, keys: &mut Vec<String>, number_of_new_results: &mut usize, number_of_stored_results: &mut usize, insert_result: Option<String>) {
    if let Some(inserted_key) = insert_result {
        *number_of_new_results += 1usize;

        let progress: Maybe<ResponseResult> = Maybe {
            data: Ok(ResponseResult::GPT3ResultStatus(GPT3ResultStatus {
                number_of_results: *number_of_new_results + *number_of_stored_results,
            })),
            timestamp: Utc::now().timestamp(),
        };
        info!("insert_progress: {:?}",progress);

        keys.push(key.to_owned());
        task_store.push(&key, progress).ok();
    } else {
        *number_of_stored_results += 1usize;
    }
}
*/

pub fn retrieve_context_from_description_and_community_link_to_text_results_for_prompt(task_store: &TaskMemoryStore, description: &str, text_triggers: Vec<String>) -> anyhow::Result<String> {

    let description_text_result =  LinkToTextResult::new(description,vec![description.to_string()],vec![vec![true]],300);
    let mut linked_text = vec![description_text_result];

    if let Some(item) = retrieve_community_portal_data_from_description(&task_store,description)? {
        linked_text.push(item);
    }

    linked_text.append(&mut retrieve_embedded_data_from_links(&task_store, description)?);


    // <
    let mut prompt_embedding = Vec::new();

    let prompt_text_result = LinkToTextResult{
        link: "text_triggers".to_string(),
        text_nodes: text_triggers,
        hierarchical_segmentation: vec![vec![true]]
    };

    for chunk in &prompt_text_result.text_nodes {
        let key_for_hash = get_key_for_gpt3(string_to_hash(&chunk), "embedding");
        let mut item = try_get_or_insert_embedding_result(&task_store, &key_for_hash, vec![chunk.to_string()])?;
        prompt_embedding.append(&mut item.result);
    }
    // >

    // Vec<Vec<(Vec<f32>,(chunk: String, link: String))>>
    let mut linked_text_embeddings = Vec::new();

    for i in 0..linked_text.len() {
            for chunk in &linked_text[i].text_nodes {
                let key_for_hash = get_key_for_gpt3(string_to_hash(&chunk), "embedding");
                let mut item = try_get_or_insert_embedding_result(&task_store, &key_for_hash, vec![chunk.clone()])?;
                linked_text_embeddings.push(item.result.into_iter().zip(vec![(chunk.to_string(),linked_text[i].link.to_string())].into_iter()).collect::<Vec<(Vec<f32>,(String,String))>>());
            }
    }

    // Vec<(Vec<f32>,(chunk: String, link: String))>
    let linked_text_embeddings = linked_text_embeddings.into_iter().flatten().collect::<Vec<(Vec<f32>,(String,String))>>();


    // Vec<(index: usize, (distance: f32,(chunk: String, link: String)))>
    let mut linked_text_embeddings = linked_text_embeddings.into_iter().map(|x| {
        let mut sum_distance = 0f32;
        for v in 0..prompt_embedding.len() {
            let distance = cosine_similarity(&x.0, &prompt_embedding[v]);
            sum_distance = distance + sum_distance;
        }
        let average_distance = sum_distance / (prompt_embedding.len() as f32);
        (average_distance,x.1)
    }).enumerate().collect::<Vec<(usize,(f32,(String,String)))>>();

    // sort by distance to prompt embedding (text_triggers)
    linked_text_embeddings.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(Ordering::Equal));

    // select the first n chunks until max char_count reached
    let mut my_selection = Vec::new();
    let mut chars: usize = 0;

    for i in 0..linked_text_embeddings.len(){
        let char_count = linked_text_embeddings[i].1.1.0.chars().count();
        if char_count + chars > 4*3500 {
            break;
        }else{
            my_selection.push(&linked_text_embeddings[i]);
            chars = char_count + chars;
        }
    }

    // sort by index (restores the reading order)
    my_selection.sort_by(|a, b| a.0.cmp(&b.0));

    let mut result = String::new();

    result.push_str(format!("<source href=\"{}\">",&my_selection[0].1.1.1).as_str());

    for i in 0..my_selection.len(){
        result.push_str(&my_selection[i].1.1.0);
        // if there is a gap between indices then it's a new item
        if i + 1 < my_selection.len() && my_selection[i].0 + 1 != my_selection[i+1].0 {
            result.push_str("<next_item/>");
        }
        // if the link changes next element is a new source
        if i + 1 < my_selection.len() && my_selection[i].1.1.1 != my_selection[i+1].1.1.1 {
            result.push_str("<source/>");
            result.push_str(format!("<source href=\"{}\">",&my_selection[i].1.1.1).as_str());
        }
    }
    result.push_str("<source/>");
    Ok(result)
}

fn cosine_similarity(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    let dot_product = vec1.iter().zip(vec2).map(|(x, y)| x * y).sum::<f32>();
    let norm1 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm2 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot_product / (norm1 * norm2)
}


pub fn get_community_portal_link(task_store: &TaskMemoryStore, link_containing_text: &str) -> anyhow::Result<Option<String>> {

    let mut extracted_links: Vec<String> = extract_links(link_containing_text);

    if !extracted_links.is_empty() {

        let key_for_link_to_community = get_key_for_gpt3(string_to_hash(link_containing_text), &format!("link_to_community{}", 0));
        let prompt = get_prompt_for_gpt3(link_containing_text, PromptKind::LINK_TO_COMMUNITY);

        let result = try_get_or_insert_chat_completion_result(&task_store, &key_for_link_to_community, &GPT_3_5_TURBO_MODEL, &SYSTEM_RETRIEVE_OPTION, &prompt, 100u16)?;

        if result.result.contains("None") || !result.result.contains("Some") {
            Ok(None)
        } else {
            extracted_links.retain(|x| result.result.contains(x));
            Ok(extracted_links.get(0).map(|x| x.to_owned()))
        }


    }else{
        Ok(None)
    }
}

pub fn retrieve_community_portal_data_from_description(task_store: &TaskMemoryStore, description: &str) -> anyhow::Result<Option<LinkToTextResult>> {

    match get_community_portal_link(task_store,description) {
        Ok(Some(link)) => {
            Ok(get_embedded_data_from_link(task_store,&link)?.ok())
        }
        _ => { 
            Ok(None)
        }
    }
}


pub fn try_get_or_insert_chat_completion_result(task_store: &TaskMemoryStore, key: &str, model_name: &str, system: &str, prompt: &str, completion_token_limit: u16) -> anyhow::Result<OpenAIGPTChatCompletionResult> {

    let mut item = task_store.get::<ResponseResult>(&key, &RetrievalMethod::GetOk);
    
    if item.is_err() {

        info!("Requesting OpenAI GPT Chat Completion for key '{}'", key);
        let result: anyhow::Result<OpenAIGPTResult> = client_send_openai_gpt_chat_completion_request("./tmp/rust_openai_gpt_tools_socket", model_name.to_owned(),system.to_owned(), prompt.to_owned(), completion_token_limit);
        debug!("Received response from OpenAI GPT: {:?}", result);

        let result: Maybe<ResponseResult> = Maybe {
            data: match result {
                Ok(data) => {
                    match data {
                        OpenAIGPTResult::ChatCompletionResult(mut item) => {
                            let mut result_split = item.result.split("\"").collect::<Vec<&str>>();
                            if result_split.len() > 1 {
                                result_split.pop();
                            }
                            item.result = result_split.join("\"");
                            item.result = item.result.trim_end_matches(|c| c != '.').to_string();
                            Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::ChatCompletionResult(item)))
                        }
                        _ => {
                            Ok(ResponseResult::OpenAIGPTResult(data))
                        }
                    }

                },
                Err(err) => Err(MaybeError::AnyhowError(err.to_string())),
            },
            timestamp: Utc::now().timestamp(),
        };
        if let Err(e) = task_store.push(key, result.clone()) {
            error!("Failed to insert GPT-3 chat completion result for {}: {:?}", key, &e);
            Err(e)?
        }
        info!("Inserted GPT-3 chat completion result for {}", key);
        item = Ok(result);
    }else{
        debug!("GPT-3 chat completion result already exists for {}", key);
    }
    match item {
        Ok(Maybe { data: Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::ChatCompletionResult(result))), .. }) => {
            Ok(result)
        }
        Ok(Maybe { data: Err(err), .. }) => {
            Err(anyhow::anyhow!(err))
        }
        Err(err) => {
            Err(anyhow::anyhow!(err))
        }
        _ => {
            error!("Error: Unreachable: incorrect ResponseResult type for {}",key);
            panic!();
        }
    }

}

pub fn try_get_or_insert_embedding_result(task_store: &TaskMemoryStore, key: &str, texts: Vec<String>) -> anyhow::Result<OpenAIGPTEmbeddingResult> {


        let mut item = task_store.get::<ResponseResult>(&key, &RetrievalMethod::GetOk);

        if item.is_err() {

            info!("Requesting OpenAI GPT embedding for key '{}'", key);
            let result: anyhow::Result<OpenAIGPTResult> = client_send_openai_gpt_embedding_request("./tmp/rust_openai_gpt_tools_socket", texts);
            debug!("Received response from OpenAI GPT: {:?}", result);

            let result: Maybe<ResponseResult> = Maybe {
                data: match result {
                    Ok(data) => Ok(ResponseResult::OpenAIGPTResult(data)),
                    Err(err) => Err(MaybeError::AnyhowError(err.to_string())),
                },
                timestamp: Utc::now().timestamp(),
            };
            if let Err(e) = task_store.push(key, result.clone()) {
                error!("Failed to insert GPT-3 embedding result for {}: {:?}", key, &e);
                Err(e)?
            }
            info!("Inserted GPT-3 embedding result for {}", key);
            item = Ok(result);
        }
        else{
            debug!("GPT-3 embedding result already exists for {}", key);
        }
        match item {
            Ok(Maybe { data: Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::EmbeddingResult(result))), .. }) => {
                Ok(result)
            }
            Ok(Maybe { data: Err(err), .. }) => {
                Err(anyhow::anyhow!(err))
            }
            Err(err) => {
                Err(anyhow::anyhow!(err))
            }
            _ => {
                error!("Error: Unreachable: incorrect ResponseResult type for {}",key);
                panic!();
            }
        }
}

