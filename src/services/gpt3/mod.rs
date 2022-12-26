use chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, GPT3Result, GPT3ResultStatus, TaskResult, FraudClassification, LinkToTextResult};
use rust_openai_gpt_tools_socket_ipc::ipc::client_send_openai_gpt_text_completion_request;
use rust_openai_gpt_tools_socket_ipc::ipc::OpenAIGPTTextCompletionResult;
use crate::services::fraud_detection::get_key_for_fraud_detection;
use crate::services::link_to_text::{extract_links, get_key_for_link_to_text, link_to_id, string_to_hash};


// note this could be done on bullet points, or summaries. as well as just the original text.
// better approach idea:


// for each task:

// go over whole text, split by \n, or by sentence?!

// -> each sentence gets an embedding.
// -> sort by distance to question

// hopefully this method will censor unimportant text, and reduce the text that way.

const GPT3_PREFIX: &str = "GPT3";

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
        PromptKind::SUMMARY => {
            format!("<instruction>{}\n\n{}\n\n</instruction><source>{}</source>\n\n<result>let brief_overview: &str  = r#\"",BIAS,PROMPTS[0],text)
        },
        PromptKind::BULLET_POINTS => {
            format!("<instruction>{}\n\n{}\n\n</instruction><source>{}</source>\n\n<result>let short_hand_notes_bullet_points = [\"",BIAS,PROMPTS[1],text)
        },
        PromptKind::LINK_TO_COMMUNITY  => {
            let distance = 200;
            let mut result = String::new();
            let links = extract_links(text);

            let mut last_link_end = 0;
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
                last_link_end += before_link.len() + link.len();
            }
            result.push_str(&text[last_link_end..]);

            format!("<instruction>{}\n\n</instruction><source>{}</source>\n\n<result>let maybe_selected_link: Option<String> = ",PROMPTS[2],result)
        }
        PromptKind::QUESTION(index) => {
            format!("<instruction>{}\n\nA short message containing the answer to Q: {}\n\n</instruction><source>{}</source>\n\n<result>let first_hand_account: &str = r#\"",BIAS,QUESTIONS[index],text)
        },
        PromptKind::COMMUNITY_NOTE(index) => {
            format!("<instruction>{}\n\nA short message containing the {}\n\n</instruction><source>{}</source>\n\n<result>let first_hand_account: &str = r#\"",BIAS,COMMUNITY_NOTES[index],text)
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

                        let bullet_points_for_each = retrieve_paragraph_to_bullet_points_results(&task_store, &description);
                        // Ok(None) -> something went wrong
                        // Err(_) -> key not yet available
                        // Ok(Some()) -> bullet point.

                        // check if bullet_points contain Err(_) if yes, then continue.

                        if bullet_points_for_each.iter().flatten().filter(|x| x.is_err()).count() == 0 {
                            let mut bullet_point_text = String::new();
                            for bullet_points in bullet_points_for_each {
                                for bullet_point in bullet_points {
                                    if let Ok(Some(bp)) = bullet_point {
                                        let lines: Vec<String> = bp
                                            .split_whitespace()
                                            .map(|x| format!("{} ", x))
                                            .collect();
                                        for line in lines {
                                            if !(bullet_point_text.len() + line.len() > 4 * 3500) {
                                                bullet_point_text.push_str(&line);
                                            } else {
                                                break; // out of space for bullet points
                                            }
                                        }
                                    }
                                }
                            }

                            // TODO: use embedding api and order by distance, then take first n elements until full.
                            // TODO: discourages duplicate / similar bullet points.
                            // for now top-down selection until size limit reached
                            // TODO: use embedding to filter points specifically for different prompts.

                            for i in 0..2 {
                                // ** this means this might be called more than once.
                                let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", i + 1));
                                let prompt = get_prompt_for_gpt3(&bullet_point_text,PromptKind::QUESTION(i));
                                let insert_result = insert_gpt3_result(&task_store, &key_for_hash, &prompt, 200u16);
                                insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result { Some(key_for_hash) } else { None });
                            }
                            for i in 0..6 {
                                // ** this means this might be called more than once.
                                let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", i + 1 + 2));
                                let prompt = get_prompt_for_gpt3(&bullet_point_text,PromptKind::COMMUNITY_NOTE(i));
                                let insert_result = insert_gpt3_result(&task_store, &key_for_hash, &prompt, 200u16);
                                insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result { Some(key_for_hash) } else { None });
                            }
                        }

                        let key_for_hash = get_key_for_gpt3(hash, &format!("briefing{}", 0));
                        let prompt = get_prompt_for_gpt3(&text,PromptKind::SUMMARY);
                        let insert_result = insert_gpt3_result(&task_store, &key_for_hash, &prompt,150u16);
                        insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result {Some(key_for_hash)}else {None});

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

pub fn retrieve_paragraph_to_bullet_points_results(task_store: &TaskMemoryStore, description: &str) -> Vec<Vec<anyhow::Result<Option<String>>>> {


    let mut bullet_points_for_each: Vec<Vec<anyhow::Result<Option<String>>>> = Vec::new();

    let mut linked_text = retrieve_link_to_text_results(&task_store,description);
    linked_text.insert(0, Ok(Some(description.to_string())));


    let max_number_of_links = 2usize;
    let max_number_of_paragraphs = 2usize;
    let max_prompt_length = 8000usize; // ~2000 tokens
    let max_prompt_output_length = 1000u16; // tokens
    // WORST CASE
    // max_number_of_links X max_number_of_paragraphs X (max_prompt_length + output tokens)
    // currently --> 6300 tokens total

    // TAKE ONLY FIRST N LINKS (actually last N LINKS within proposal)
    for i in 0..std::cmp::min(max_number_of_links,linked_text.len()) {

            let mut bullet_points_for_text: Vec<anyhow::Result<Option<String>>> = Vec::new();

            if let Ok(Some(text))= &linked_text[i] {


                let split_whitespace = text.split_whitespace()
                    .map(|x| format!("{} ", x))
                    .collect::<Vec<String>>();

                // now go over the split_paragraphs and build your string until size exhausted.
                // this way the final string will end naturally most of the time.

                let mut size_limited_paragraphs: Vec<String> = Vec::new();

                let mut paragraph = String::new();

                for word in split_whitespace {
                    if paragraph.len() + word.len() > max_prompt_length {
                        size_limited_paragraphs.push(paragraph);
                        paragraph = String::new();
                    }
                    paragraph.push_str(&word);
                }
                size_limited_paragraphs.push(paragraph);

                // TAKE ONLY FIRST N Paragraphs
                size_limited_paragraphs = size_limited_paragraphs.into_iter().take(max_number_of_paragraphs).collect();

                for each in &size_limited_paragraphs {
                    let key_for_hash = get_key_for_gpt3(string_to_hash(each), "bullet_point");
                    let prompt = get_prompt_for_gpt3(&each,PromptKind::BULLET_POINTS);
                    insert_gpt3_result(&task_store, &key_for_hash, &prompt,  max_prompt_output_length);
                }

                for each in &size_limited_paragraphs {
                    let key_for_hash = get_key_for_gpt3(string_to_hash(each), "bullet_point");
                    if task_store.contains_key(&key_for_hash) {
                        match task_store.get::<ResponseResult>(&key_for_hash, &RetrievalMethod::GetOk) {
                            Ok(Maybe { data: Ok(ResponseResult::GPT3Result(GPT3Result { result, .. })), .. }) => {
                                bullet_points_for_text.push(Ok(Some(result)));
                            }
                            Err(err) => { bullet_points_for_text.push(Err(anyhow::anyhow!(err))); }
                            _ => { bullet_points_for_text.push(Ok(None)); }
                        }
                    }
                }
            }
            else if let Err(err)= &linked_text[i] { // value not yet available
                bullet_points_for_text.push(Err(anyhow::anyhow!(err.to_string())));
            }
           else if let Ok(None)= &linked_text[i] { // error case
               bullet_points_for_text.push(Ok(None));
            }
        bullet_points_for_each.push(bullet_points_for_text);
    }
    bullet_points_for_each
}


pub fn retrieve_link_to_text_results(task_store: &TaskMemoryStore, description: &str) -> Vec<anyhow::Result<Option<String>>> {

    let key_for_link_to_community = get_key_for_gpt3(string_to_hash(description), &format!("link_to_community{}", 0));
    let prompt = get_prompt_for_gpt3(description,PromptKind::LINK_TO_COMMUNITY);
    insert_gpt3_result(&task_store, &key_for_link_to_community, &prompt,100u16);

    let mut extracted_links = extract_links(description);

    if task_store.contains_key(&key_for_link_to_community) {
        match task_store.get::<ResponseResult>(&key_for_link_to_community, &RetrievalMethod::GetOk) {
            Ok(Maybe { data: Ok(ResponseResult::GPT3Result(GPT3Result { result, .. })), .. }) => {
                if result.contains("None") || !result.contains("Some"){
                    extracted_links = Vec::new();
                }else{
                    extracted_links.retain(|x| result.contains(x));
                }
            }
            Err(err) => { }
            _ => { }
        }
    }

    let link_keys = extracted_links.iter().map(|x| get_key_for_link_to_text(&link_to_id(x))).collect::<Vec<String>>();

    let mut texts: Vec<anyhow::Result<Option<String>>> = Vec::new();

    for link_key in link_keys {
        if task_store.contains_key(&link_key) {
            match task_store.get::<ResponseResult>(&link_key, &RetrievalMethod::GetOk) {
                Ok(Maybe { data: Ok(ResponseResult::LinkToTextResult(LinkToTextResult { link, text, .. })), .. }) => {
                     texts.push(Ok(Some(text)));
                }
                Err(err) => { texts.push(Err(anyhow::anyhow!(err))); }
                _ => { texts.push(Ok(None)); }
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

pub fn insert_gpt3_result(task_store: &TaskMemoryStore, key: &str, prompt: &str, completion_token_limit: u16) -> bool {

    if !task_store.contains_key(&key) {
        
        error!("client_send_openai_gpt_summarization_request");
        let result: anyhow::Result<OpenAIGPTTextCompletionResult> = client_send_openai_gpt_text_completion_request("./tmp/rust_openai_gpt_tools_socket", prompt.to_owned(), completion_token_limit);
        error!("OpenAIGPTSummarizationResult: {:?}",result);

        let result: Maybe<ResponseResult> = Maybe {
            data: match result {
                Ok(data) => Ok(ResponseResult::GPT3Result(GPT3Result {
                    prompt: data.request.prompt,
                    result: data.result.replace("\"#;","").replace("\n#","").replace(". #;","")
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