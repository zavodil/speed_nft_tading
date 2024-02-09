use near_contract_standards::fungible_token::Balance;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedMap},
    env, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, BorshStorageKey, PanicOnDefault, PromiseOrValue, Timestamp,
};
use std::convert::TryFrom;

use crate::utils::FeeFraction;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata,
};
use near_contract_standards::non_fungible_token::{
    NonFungibleToken, NonFungibleTokenEnumeration, Token, TokenId,
};
use near_sdk::json_types::U128;
use serde_json::Value;

mod utils;

pub const TIMESTAMP_MAX_INTERVAL: u64 = 5 * 60 * 1_000_000_000;

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    NonFungibleToken,
    ContractMetadata,
    TokenMetadata,
    Enumeration,
    Approval,
    TokenMetadataTemplate,
    InternalBalances,
    TokenPrices,
    Referrals,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    owner_id: AccountId,
    public_key: String,
    min_mint_price: Balance,

    tokens: NonFungibleToken,
    contract_metadata: LazyOption<NFTContractMetadata>,
    token_metadata: LazyOption<TokenMetadata>,

    internal_balances: UnorderedMap<AccountId, Balance>,
    token_prices: UnorderedMap<TokenId, Balance>,
    referrals: UnorderedMap<AccountId, AccountId>,
    mint_price_increase_fee: FeeFraction,
    seller_fee: FeeFraction,
    referral_1_fee: FeeFraction,
    referral_2_fee: FeeFraction,
}

#[near_bindgen]
impl Contract {
    #[init]
    // mint_price_increase_fee - how much price grows on new resale
    // seller_fee - fee of profit for prev owner
    // referral_fee - fee of profit (new_price - old_price) for referrals
    pub fn new(
        owner_id: AccountId,
        public_key: String,
        min_mint_price: U128,
        mint_price_increase_fee: FeeFraction,
        seller_fee: FeeFraction,
        referral_1_fee: FeeFraction,
        referral_2_fee: FeeFraction,
        contract_metadata: NFTContractMetadata,
        token_metadata: TokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        contract_metadata.assert_valid();
        token_metadata.assert_valid();
        mint_price_increase_fee.assert_valid();
        seller_fee.assert_valid();
        referral_1_fee.assert_valid();
        referral_2_fee.assert_valid();

        Self {
            owner_id: owner_id.clone(),
            public_key,
            min_mint_price: min_mint_price.0,
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            contract_metadata: LazyOption::new(
                StorageKey::ContractMetadata,
                Some(&contract_metadata),
            ),
            token_metadata: LazyOption::new(
                StorageKey::TokenMetadataTemplate,
                Some(&token_metadata),
            ),
            internal_balances: UnorderedMap::new(StorageKey::InternalBalances),
            token_prices: UnorderedMap::new(StorageKey::TokenPrices),
            referrals: UnorderedMap::new(StorageKey::Referrals),
            mint_price_increase_fee,
            seller_fee,
            referral_1_fee,
            referral_2_fee,
        }
    }

    // message - a stringified JSON Object {"token_id": "<ipfs_hash>", "account_id": "name.near", "referral_id": "ref.near", timestamp: Timestamp}
    // sig_string - message signed with self.public_key
    #[payable]
    pub fn nft_mint(&mut self, message: String, sig_string: String) -> Token {
        let mut pk = [0u8; 32];
        let _pk_bytes = hex::decode_to_slice(self.public_key.clone(), &mut pk as &mut [u8]);

        let mut sig = [0u8; 64];
        let _sig_string = hex::decode_to_slice(sig_string, &mut sig as &mut [u8]);

        if verification(&pk, &message, &sig) {
            let parsed_data: Result<Value, serde_json::Error> = serde_json::from_str(&message);

            match parsed_data {
                Ok(parsed_json) => {
                    let receiver_id = env::predecessor_account_id();
                    let account_id: TokenId = parsed_json["account_id"]
                        .as_str()
                        .expect("token_id missed")
                        .parse()
                        .unwrap();
                    assert_eq!(receiver_id, account_id, "Mint for yourself only");

                    let token_id: TokenId = parsed_json["token_id"]
                        .as_str()
                        .expect("token_id missed")
                        .parse()
                        .unwrap();

                    let timestamp: Timestamp = parsed_json["timestamp"]
                        .as_str()
                        .expect("timestamp missed")
                        .parse()
                        .unwrap();

                    assert!(
                        timestamp + TIMESTAMP_MAX_INTERVAL >= env::block_timestamp(),
                        "Timestamp is too old"
                    );

                    if let Some(referral_id) = parsed_json["referral_id"].as_str() {
                        assert_ne!(referral_id, receiver_id, "Can't refer yourself");
                        self.referrals.insert(
                            &receiver_id,
                            &AccountId::try_from(referral_id.to_string()).unwrap(),
                        );
                        // TODO emit event new referral
                    }

                    let deposit: Balance = env::attached_deposit().as_yoctonear();

                    if let Some(token) = self.tokens.nft_token(token_id.clone()) {
                        // token already exists
                        let old_price: Balance = self
                            .token_prices
                            .get(&token_id)
                            .expect("Token price is missing");
                        let profit = self.mint_price_increase_fee.multiply(old_price);
                        let new_price = old_price + profit;

                        assert!(deposit >= new_price, "Illegal deposit");

                        // distribute seller reward
                        let seller_id: AccountId = token.owner_id.clone();
                        assert_ne!(seller_id, receiver_id, "Current and next owner must differ");

                        let seller_fee: Balance = self.seller_fee.multiply(profit);
                        self.internal_add_balance(&seller_id, seller_fee);

                        // distribute affiliate reward
                        let mut referral_1_fee: Balance = 0;
                        let mut referral_2_fee: Balance = 0;
                        if let Some(referral_1) = self.referrals.get(&receiver_id) {
                            referral_1_fee = self.referral_1_fee.multiply(profit);
                            self.internal_add_balance(&referral_1, referral_1_fee);
                            if let Some(referral_2) = self.referrals.get(&referral_1) {
                                referral_2_fee = self.referral_2_fee.multiply(profit);
                                self.internal_add_balance(&referral_2, referral_2_fee);
                            }
                        }

                        // distribute system reward
                        let mut system_fee = Some(profit);
                        for val in &[seller_fee, referral_1_fee, referral_2_fee] {
                            match system_fee {
                                Some(r) => {
                                    system_fee = r.checked_sub(*val);
                                    if system_fee.is_none() {
                                        break; // Exit loop if overflow occurs
                                    }
                                }
                                None => {
                                    break; // Exit loop if previous subtraction overflowed
                                }
                            }
                        }
                        if let Some(system_fee) = system_fee {
                            self.internal_add_balance(&self.owner_id.clone(), system_fee);
                        }

                        self.tokens.internal_transfer(&seller_id, &receiver_id, &token_id, None, None);

                        token.clone()
                    } else {
                        self.token_prices.insert(&token_id, &self.min_mint_price);
                        let token_metadata = self.get_token_metadata(&token_id);

                        self.tokens
                            .internal_mint(token_id, receiver_id, Some(token_metadata))
                    }
                }
                Err(e) => {
                    env::panic_str(&format!("Error parsing JSON: {:?}", e));
                }
            }
        } else {
            env::panic_str("Signature check failed")
        }
    }
}

impl Contract {
    pub fn get_token_metadata(&self, token_id: &TokenId) -> TokenMetadata {
        let mut token_metadata = self.token_metadata.get().unwrap();
        token_metadata.media = Some(format!("ipfs://{}", token_id));

        token_metadata
    }
}

#[near_bindgen]
impl NonFungibleTokenCore for Contract {
    #[payable]
    #[allow(unused)]
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        env::panic_str("Not allowed")
    }

    #[payable]
    #[allow(unused)]
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        env::panic_str("Not allowed")
    }

    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        self.tokens.nft_token(token_id)
    }
}

#[near_bindgen]
impl NonFungibleTokenEnumeration for Contract {
    fn nft_total_supply(&self) -> near_sdk::json_types::U128 {
        self.tokens.nft_total_supply()
    }

    fn nft_tokens(
        &self,
        from_index: Option<near_sdk::json_types::U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        self.tokens.nft_tokens(from_index, limit)
    }

    fn nft_supply_for_owner(&self, account_id: AccountId) -> near_sdk::json_types::U128 {
        self.tokens.nft_supply_for_owner(account_id)
    }

    fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<near_sdk::json_types::U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        self.tokens
            .nft_tokens_for_owner(account_id, from_index, limit)
    }
}

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.contract_metadata.get().unwrap()
    }
}

fn verification(pk_string: &[u8; 32], message: &str, sig_string: &[u8; 64]) -> bool {
    env::ed25519_verify(sig_string, message.as_bytes(), pk_string)
}
