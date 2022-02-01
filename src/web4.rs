use crate::*;
use near_contract_standards::non_fungible_token::metadata::{NFTContractMetadata, TokenMetadata};
use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::json_types::Base64VecU8;
use near_sdk::{env, serde_json};
use std::collections::HashMap;
use std::str::FromStr;

const INDEX_BODY: &str = include_str!("../res/index.html");
const IPFS_PREFIX: &str = "https://cloudflare-ipfs.com/ipfs";

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Web4Request {
    #[serde(rename = "accountId")]
    account_id: Option<AccountId>,
    path: String,
    params: Option<HashMap<String, String>>,
    query: Option<HashMap<String, Vec<String>>>,
    preloads: Option<HashMap<String, Web4Response>>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct Web4Response {
    #[serde(rename = "contentType")]
    content_type: Option<String>,
    status: Option<u32>,
    body: Option<Base64VecU8>,
    #[serde(rename = "bodyUrl")]
    body_url: Option<String>,
    #[serde(rename = "preloadUrls")]
    preload_urls: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub metadata: Option<TokenMetadata>,
}

impl Web4Response {
    pub fn html_response(text: String) -> Self {
        Self {
            content_type: Some(String::from("text/html; charset=UTF-8")),
            body: Some(text.into_bytes().into()),
            ..Default::default()
        }
    }

    pub fn plain_response(text: String) -> Self {
        Self {
            content_type: Some(String::from("text/plain; charset=UTF-8")),
            body: Some(text.into_bytes().into()),
            ..Default::default()
        }
    }

    pub fn preload_urls(urls: Vec<String>) -> Self {
        Self {
            preload_urls: Some(urls),
            ..Default::default()
        }
    }

    pub fn body_url(url: String) -> Self {
        Self {
            body_url: Some(url),
            ..Default::default()
        }
    }

    pub fn status(status: u32) -> Self {
        Self {
            status: Some(status),
            ..Default::default()
        }
    }
}

fn filter_string(s: String) -> String {
    s.chars()
        .into_iter()
        .filter_map(|c| match c {
            '\n' => Some(' '),
            ' ' | '_' | '.' | '-' | ',' | '!' | '(' | ')' | '/' | '=' | ':' | '+' | '?' | '#'
            | '%' | '|' => Some(c),
            _ if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

#[near_bindgen]
impl Contract {
    #[allow(unused_variables)]
    pub fn web4_get(&self, request: Web4Request) -> Web4Response {
        let path = request.path;

        if path == "/robots.txt" {
            return Web4Response::plain_response("User-agent: *\nDisallow:".to_string());
        }

        let (nft_account_id, token_id) = path[1..].split_once('/').expect("Token ID is missing");
        let nft_account_id = AccountId::from_str(nft_account_id).expect("Invalid NFT account ID");
        let nft_metadata_url =
            format!("/web4/contract/{}/nft_metadata", nft_account_id.to_string());
        let token_url = format!(
            "/web4/contract/{}/nft_token?token_id={}",
            nft_account_id.to_string(),
            token_id
        );

        if let Some(preloads) = request.preloads {
            let token: Token = serde_json::from_slice(
                &preloads
                    .get(&token_url)
                    .unwrap()
                    .body
                    .as_ref()
                    .expect("Token not found")
                    .0,
            )
            .expect("Failed to parse token");
            let nft_metadata: NFTContractMetadata = serde_json::from_slice(
                &preloads
                    .get(&nft_metadata_url)
                    .unwrap()
                    .body
                    .as_ref()
                    .expect("NFT Metadata doesn't exist")
                    .0,
            )
            .expect("Failed to parse NFT Metadata");
            let token_metadata = token.metadata.expect("Token metadata is missing");
            let token_media = token_metadata.media.unwrap_or_default();

            let image_url = if let Some(base_uri) = &nft_metadata.base_uri {
                format!("{}/{}", base_uri, token_media)
            } else if token_media.starts_with("Qm") {
                format!("{}/{}", IPFS_PREFIX, token_media)
            } else {
                token_media
            };

            let token_title = filter_string(token_metadata.title.unwrap_or_default());

            Web4Response::html_response(
                INDEX_BODY
                    .replace("%IMAGE%", &filter_string(image_url))
                    .replace("%TITLE%", &token_title)
                    .replace(
                        "%URL_TITLE%",
                        &filter_string(format!("{} | {}", token_title, nft_metadata.name)),
                    )
                    .replace(
                        "%DESCRIPTION%",
                        &filter_string(
                            token_metadata
                                .description
                                .unwrap_or_else(|| nft_metadata.name),
                        ),
                    )
                    .replace("%OWNER_ID%", &token.owner_id.to_string())
                    .replace("%CONTRACT_ID%", &nft_account_id.to_string())
                    .replace(
                        "%URL%",
                        &format!("{}.page{}", env::current_account_id().to_string(), path),
                    ),
            )
        } else {
            Web4Response::preload_urls(vec![nft_metadata_url, token_url])
        }
        //
        // let path = request.path.expect("Path expected");
        // if path.starts_with("/static/") || path == "/favicon.png" || path == "/manifest.json" {
        //     return Web4Response::body_url(
        //         String::from("ipfs://bafybeigifbsj3nnbufxa3non7xas23r3yqjlfx3v3k27qgdgch2mmqdeue")
        //             + &path,
        //     );
        // }
        //
        // if path == "/robots.txt" {
        //     return Web4Response::plain_response("User-agent: *\nDisallow:".to_string());
        // }
        //
        // let article_id = path
        //     .rfind('/')
        //     .map(|p| path[(p + 1)..].to_string())
        //     .unwrap_or_default();
        //
        // let escaped_article_id = filter_string(article_id.clone());
        //
        // let title = if path.starts_with(PREFIX_HISTORY) {
        //     format!("Edit history of {} | wiki", escaped_article_id)
        // } else {
        //     format!("{} | wiki", escaped_article_id)
        // };
        //
        // let article = self.internal_get_article(&article_id);
        //
        // let description = article
        //     .map(|article| filter_string(article.body))
        //     .unwrap_or_else(|| "the wiki built on NEAR".to_string());
        //
        // Web4Response::html_response(
        //     INDEX_BODY
        //         .replace("The wiki", &title)
        //         .replace("the wiki built on NEAR", &description),
        // )
    }
}
