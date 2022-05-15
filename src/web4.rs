use std::cmp::max;
use crate::*;
use crate::utils::*;

use near_sdk::json_types::Base64VecU8;
use near_sdk::{env, serde_json, Balance, Timestamp};
use std::collections::HashMap;

const INDEX_BODY: &str = include_str!("../res/index.html");

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
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

pub type DurationSec = u32;

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PriceData {
    #[serde(with = "u64_dec_format")]
    pub timestamp: Timestamp,
    pub recency_duration_sec: DurationSec,

    pub prices: Vec<AssetOptionalPrice>,
}

#[derive(Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetPrice {
    #[serde(with = "u128_dec_format")]
    multiplier: Balance,
    decimals: u8,
}

impl AssetOptionalPrice {
    pub fn get_price(&self) -> String {
        if let Some(data) = &self.price {
            data.multiplier.to_string()
        } else {
            "Not found".to_string()
        }
    }
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetOptionalPrice {
    pub asset_id: AccountId,
    pub price: Option<AssetPrice>,
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Asset {
    pub reports: Vec<Report>,
    pub emas: Vec<AssetEma>,
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Report {
    pub oracle_id: AccountId,
    #[serde(with = "u64_dec_format")]
    pub timestamp: Timestamp,
    pub price: AssetPrice,
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetEma {
    pub period_sec: DurationSec,
    #[serde(with = "u64_dec_format")]
    pub timestamp: Timestamp,
    pub price: Option<AssetPrice>,
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

#[near_bindgen]
impl Contract {
    #[allow(unused_variables)]
    pub fn web4_get(&self, request: Web4Request) -> Web4Response {
        let path = request.path;

        if path == "/robots.txt" {
            return Web4Response::plain_response("User-agent: *\nDisallow:".to_string());
        }

        let get_price_data_path =
            format!("/web4/contract/{}/get_price_data", PRICE_ORACLE.to_string());

        let get_assets_path =
            format!("/web4/contract/{}/get_assets", PRICE_ORACLE.to_string());

        if let Some(preloads) = request.preloads {
            let price_data: PriceData = serde_json::from_slice(
                &preloads
                    .get(&get_price_data_path)
                    .unwrap()
                    .body
                    .as_ref()
                    .expect("Data not found")
                    .0,
            )
                .expect("Failed to parse price data");

            let assets: Vec<(AccountId, Asset)> = serde_json::from_slice(
                &preloads
                    .get(&get_assets_path)
                    .unwrap()
                    .body
                    .as_ref()
                    .expect("Data not found")
                    .0,
            )
                .expect("Failed to parse assets");

            let mut prices_table: String = "".to_string();
            for price_data in &price_data.prices {
                if let Some(config) = self.config.get(&price_data.asset_id) {
                    if let Some(asset_price) = &price_data.price {
                        let price: f64 = asset_price.multiplier as f64 /
                            (10u128.pow((asset_price.decimals - config.decimals) as u32)) as f64;
                        prices_table = format!("{}<tr><td>{}</td><td>{}</td></tr>", &prices_table,
                                               config.token_name,
                                               price.to_string());
                    }
                } else {
                    if price_data.asset_id.to_string() != "c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2.factory.bridge.near" {
                        prices_table = format!("{}<tr><td>{}</td><td>{}</td></tr>", &prices_table,
                                               &price_data.asset_id,
                                               &price_data.get_price());
                    }
                }
            }

            let mut validators: HashMap<AccountId, Timestamp> = HashMap::new();
            for (account_id, asset) in assets {
                for report in asset.reports {
                    let last_timestamp = max(
                        report.timestamp,
                        validators.get(&report.oracle_id).cloned().unwrap_or_default());
                    validators.insert(report.oracle_id, last_timestamp);
                }

                for ema in asset.emas {
                    if let Some(config) = self.config.get(&account_id) {
                        if let Some(ema_price) = &ema.price {
                            let price: f64 = ema_price.multiplier as f64 /
                                (10u128.pow((ema_price.decimals - config.decimals) as u32)) as f64;
                            prices_table = format!("{}<tr><td>{} EMA#{}</td><td>{}</td></tr>", &prices_table,
                                                   config.token_name,
                                                   ema.period_sec.to_string(),
                                                   ((price * 10000f64).floor() / 10000.0).to_string());
                        }
                    }
                }
            }

            let mut validators_table: String = "".to_string();
            for (validator_id, timestamp) in validators {
                let time_diff: Timestamp = env::block_timestamp() - timestamp;
                validators_table = format!("{}<tr><td>{}</td><td>{:.2}</td></tr>", &validators_table,
                                           &validator_id,
                                           &(time_diff as f64 / 10u64.pow(9) as f64));
            }

            Web4Response::html_response(
                INDEX_BODY
                    .replace("%PRICES%", &prices_table)
                    .replace("%VALIDATORS%", &validators_table),
            )
        } else {
            Web4Response::preload_urls(vec![get_assets_path, get_price_data_path])
        }
    }
}
