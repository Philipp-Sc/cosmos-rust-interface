use chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, GPT3Result, GPT3ResultStatus, TaskResult, FraudClassification, LinkToTextResult};
use rust_openai_gpt_tools_socket_ipc::ipc::client_send_openai_gpt_summarization_request;
use rust_openai_gpt_tools_socket_ipc::ipc::OpenAIGPTSummarizationResult;
use crate::services::fraud_detection::get_key_for_fraud_detection;
use crate::services::link_to_text::{extract_links, get_key_for_link_to_text, link_to_id, string_to_hash};

// AWAIT: fraud_detection
// AWAIT: link_to_text

//

// STEP 2) for all the texts create a bullet point summary. this reduces the text length, token amount.
// STEP 3) then merge the bullet points
// STEP 4) run the original gpt3 prompt on them.

const GPT3_PREFIX: &str = "GPT3";


// explain paper
// explain this part of the proposal.

/*

What problem is it attempting to solve, and how does it propose to do so?
What are the potential benefits of the proposal? How will it improve the operation of the cryptocurrency or blockchain in question?
What are the potential risks or downsides of the proposal? What unintended consequences might it have, and how might these be mitigated?
Who is behind the proposal? What is their background and experience in the crypto space, and what is their motivation for making the proposal?
How would the proposal be funded? Would it require the allocation of new tokens, or the use of existing funds?
How would the proposal be implemented? What technical changes would be required, and how would they be implemented?

"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Feasibility and technical viability",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Economic impact",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Legal and regulatory compliance",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Long-term sustainability",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Transparency & Accountability",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Community Support",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Risks",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Benefits",
"Community Notes aim to create a better informed world by empowering people to collaboratively add context to potentially misleading proposals. Contributors can leave notes on proposal and if enough contributors from different points of view rate that note as helpful, the note will be publicly shown. Following is a truthful note for this proposal on Recommendations or advice",

*/

const SUMMARY_PROMPT: &str ="Provide a brief overview of the motivation or purpose behind this governance proposal. Tweet.";

const PROMPTS: [&str;6] = [

            "What problem is it attempting to solve, and how does it propose to do so?",
            "What are the potential benefits of the proposal? How will it improve the operation of the cryptocurrency or blockchain in question?",
            "What are the potential risks or downsides of the proposal? What unintended consequences might it have, and how might these be mitigated?",
            "Who is behind the proposal? What is their background and experience in the crypto space, and what is their motivation for making the proposal?",
            "How would the proposal be funded? Would it require the allocation of new tokens, or the use of existing funds?",
            "How would the proposal be implemented? What technical changes would be required, and how would they be implemented?"
               ];

pub fn get_key_for_gpt3(hash: u64, prompt_id: &str) -> String {
    format!("{}_{}_{}",GPT3_PREFIX, prompt_id, hash)
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

                        let bullet_points = retrieve_paragraph_to_bullet_points_results(&task_store, &description);

                        let mut bullet_point_text = String::new();
                        for bullet_point in bullet_points {
                            if let Some(bp) = bullet_point {
                                let lines: Vec<&str> = bp
                                    .split("•")
                                    .flat_map(|line| line.split("\\n"))
                                    .flat_map(|line| line.split("\n"))
                                    .collect();
                                for line in lines {
                                    if !(bullet_point_text.len() + line.len() > 4 * 3000) {
                                        bullet_point_text.push_str(line);
                                        bullet_point_text.push_str("\n");
                                    } else {
                                        break; // out of space for bullet points
                                    }
                                }
                            }
                        }
                        // TODO: use embedding api and order by distance, then take first n elements until full.
                        // TODO: discourages duplicate / similar bullet points.
                        // for now top-down selection until size limit reached
                        // TODO: use embedding to filter points specifically for different prompts.


                        // first get the summary
                        let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", 0));
                        let insert_result = insert_gpt3_result(&task_store, &key_for_hash, &text, SUMMARY_PROMPT,100u16);
                        insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result {Some(key_for_hash)}else {None});


                        for i in 0..PROMPTS.len() {

                            let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", i+1));
                            let insert_result = insert_gpt3_result(&task_store, &key_for_hash, &bullet_point_text, PROMPTS[i],100u16);
                            insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result {Some(key_for_hash)}else {None});
                            
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

pub fn retrieve_paragraph_to_bullet_points_results(task_store: &TaskMemoryStore, description: &str) -> Vec<Option<String>> {

    let mut texts: Vec<Option<String>> = Vec::new();


    let mut linked_text = retrieve_link_to_text_results(&task_store,description);
    linked_text.insert(0, Some(description.to_string()));


    let max_number_of_links = 3usize;
    let max_number_of_paragraphs = 6usize;
    let max_prompt_length = 1000usize; // ~250 tokens
    let max_prompt_output_length = 100u16; // tokens
    // WORST CASE
    // max_number_of_links X max_number_of_paragraphs X (max_prompt_length + output tokens)
    // currently --> 6300 tokens total

    // TAKE ONLY FIRST N LINKS (actually last N LINKS within proposal)
    for i in 0..std::cmp::min(max_number_of_links,linked_text.len()) {

        if let Some(text) = &linked_text[i] {

            let p = text.replace("\n","\\n").split("\\n").map(|x| x.to_string()).collect::<Vec<String>>();
            let split_paragraphs = p.iter().map(|x| x.chars().collect::<Vec<char>>().chunks(1024).map(|chunk| chunk.iter().collect::<String>()).collect::<Vec<String>>()).collect::<Vec<Vec<String>>>();

            // now go over the split_paragraphs and build your string until size exhausted.
            // this way the final string will end naturally most of the time.
            
            let mut size_limited_paragraphs: Vec<String> = Vec::new();

            for paragraphs in split_paragraphs {
                let mut resulting_string = String::new();
                for paragraph in paragraphs {
                    if resulting_string.len() + paragraph.len() > max_prompt_length {
                        size_limited_paragraphs.push(resulting_string);
                        resulting_string = String::new();
                    }
                    resulting_string.push_str(&paragraph);
                }
                size_limited_paragraphs.push(resulting_string);
            }
            // TAKE ONLY FIRST N Paragraphs
            size_limited_paragraphs = size_limited_paragraphs.into_iter().take(max_number_of_paragraphs).collect();

            for each in &size_limited_paragraphs {

                let key_for_hash = get_key_for_gpt3(string_to_hash(each), "bullet_point");
                insert_gpt3_result(&task_store, &key_for_hash, each, "Reiterate the all points present in a concise and clear manner, removing any unnecessary information, noise or filler words. Presented as bullet points.",max_prompt_output_length);
            }

            for each in &size_limited_paragraphs {

                let key_for_hash = get_key_for_gpt3(string_to_hash(each), "bullet_point");
                if task_store.contains_key(&key_for_hash) {
                    match task_store.get::<ResponseResult>(&key_for_hash, &RetrievalMethod::GetOk) {
                        Ok(Maybe { data: Ok(ResponseResult::GPT3Result(GPT3Result { result, .. })), .. }) => {
                            texts.push(Some(result));
                        }
                        Err(_) => { texts.push(None); }
                        _ => { texts.push(None); }
                    }
                }
            }
        }
    }
    texts
}


pub fn retrieve_link_to_text_results(task_store: &TaskMemoryStore, description: &str) -> Vec<Option<String>> {

    let link_keys = extract_links(description).iter().map(|x| get_key_for_link_to_text(&link_to_id(x))).collect::<Vec<String>>();

    let mut texts: Vec<Option<String>> = Vec::new();

    for link_key in link_keys {
        if task_store.contains_key(&link_key) {
            match task_store.get::<ResponseResult>(&link_key, &RetrievalMethod::GetOk) {
                Ok(Maybe { data: Ok(ResponseResult::LinkToTextResult(LinkToTextResult { link, text, .. })), .. }) => {
                     texts.push(Some(text));
                }
                Err(_) => { texts.push(None); }
                _ => { texts.push(None); }
            }
        }
    }
    texts
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

pub fn insert_gpt3_result(task_store: &TaskMemoryStore, key: &str, text: &str, prompt: &str, completion_token_limit: u16) -> bool {



    if !task_store.contains_key(&key) {
        
        error!("client_send_openai_gpt_summarization_request");
        let result: anyhow::Result<OpenAIGPTSummarizationResult> = client_send_openai_gpt_summarization_request("./tmp/rust_openai_gpt_tools_socket", text.to_owned(), prompt.to_owned(),completion_token_limit);
        error!("OpenAIGPTSummarizationResult: {:?}",result);

        let result: Maybe<ResponseResult> = Maybe {
            data: match result {
                Ok(data) => Ok(ResponseResult::GPT3Result(GPT3Result {
                    text: data.request.text,
                    prompt: data.request.prompt,
                    result: data.result
                })),
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