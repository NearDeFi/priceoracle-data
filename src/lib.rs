mod web4;
mod utils;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId, BorshStorageKey, PanicOnDefault};
use near_sdk::collections::LookupMap;

const PRICE_ORACLE: &str = "priceoracle.near";

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub config: LookupMap<AccountId, TokenConfig>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenConfig {
    pub token_name: String,
    pub decimals: u8,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Config
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            config: LookupMap::new(StorageKey::Config)
        }
    }

    #[private]
    pub fn add_token_config(&mut self, account_id: AccountId, config: TokenConfig) {
        self.config.insert(&account_id, &config);
    }

    #[private]
    pub fn add_token_configs(&mut self, configs: Vec<(AccountId, TokenConfig)>) {
        for (account_id, config) in configs {
            self.config.insert(&account_id, &config);
        }
    }

    pub fn get_config(&self, keys: Vec<AccountId>) -> Vec<(AccountId, Option<TokenConfig>)> {
        keys
            .into_iter()
            .map(|account_id| (account_id.clone(), self.config.get(&account_id)))
            .collect()
    }
}