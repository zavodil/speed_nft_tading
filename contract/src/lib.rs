use crate::utils::FeeFraction;
use near_contract_standards::fungible_token::Balance;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata,
};
use near_contract_standards::non_fungible_token::{
    NonFungibleToken, NonFungibleTokenEnumeration, Token, TokenId,
};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedMap},
    env,
    json_types::U128,
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, BorshStorageKey, PanicOnDefault, PromiseOrValue, Timestamp,
};
use near_sdk::store::LookupMap;
use nft::*;

mod nft;
mod utils;

pub const TIMESTAMP_MAX_INTERVAL: u64 = 5 * 60 * 1_000_000_000;

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    NonFungibleToken,
    ContractMetadata,
    Enumeration,
    Approval,
    TokenMetadataTemplate,
    InternalBalances,
    StoreUserTokens,
    TokenPrices,
    TokenLastSale,
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
    store_user_tokens: LookupMap<AccountId, bool>,
    token_prices: UnorderedMap<TokenId, Balance>,
    token_last_sale: UnorderedMap<TokenId, Timestamp>,
    mint_price_increase_fee: FeeFraction,
    seller_fee: FeeFraction,
    referral_1_fee: FeeFraction,
    referral_2_fee: FeeFraction,
}

#[derive(Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, Serialize))]
#[serde(crate = "near_sdk::serde")]
pub enum MintNftMsg {
    SimpleMint {
        token_id: TokenId,
        account_id: AccountId,
        referral_1: Option<AccountId>,
        referral_2: Option<AccountId>,
        timestamp: Timestamp
    }
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
            tokens: nft_without_metadata(
                StorageKey::NonFungibleToken,
                owner_id,
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
            store_user_tokens: LookupMap::new(StorageKey::StoreUserTokens),
            token_prices: UnorderedMap::new(StorageKey::TokenPrices),
            token_last_sale: UnorderedMap::new(StorageKey::TokenLastSale),
            mint_price_increase_fee,
            seller_fee,
            referral_1_fee,
            referral_2_fee,
        }
    }



    // message - a stringified JSON Object {"token_id": "<ipfs_hash>", "account_id": "name.near", "referral_id_1": "ref.near",  "referral_id_2": "ref.near", timestamp: Timestamp}
    // sig_string - message signed with self.public_key
    #[payable]
    pub fn nft_mint(&mut self, message: String, sig_string: String) -> Token {
        let mut pk = [0u8; 32];
        let _pk_bytes = hex::decode_to_slice(self.public_key.clone(), &mut pk as &mut [u8]);

        let mut sig = [0u8; 64];
        let _sig_string = hex::decode_to_slice(sig_string, &mut sig as &mut [u8]);

        assert!(verification(&pk, &message, &sig), "Signature check failed");

        let parsed_message = serde_json::from_str::<MintNftMsg>(&message).expect("Wrong message format");

        match parsed_message {
            MintNftMsg::SimpleMint {
                token_id, account_id, referral_1, referral_2, timestamp
            } => {
                let receiver_id = env::predecessor_account_id();
                assert_eq!(receiver_id, account_id, "Mint for yourself only");

                if let Some(token_last_sale) = self.token_last_sale.get(&token_id) {
                    assert!(
                        timestamp >= token_last_sale,
                        "Timestamp is older then last sale"
                    );
                }

                assert!(
                    timestamp + TIMESTAMP_MAX_INTERVAL >= env::block_timestamp(),
                    "Timestamp is too old"
                );

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
                    self.internal_add_balance(&seller_id, old_price + seller_fee);

                    // distribute affiliate reward
                    let mut referral_1_fee: Balance = 0;
                    let mut referral_2_fee: Balance = 0;
                    if let Some(referral_1) = referral_1 {
                        referral_1_fee = self.referral_1_fee.multiply(profit);
                        self.internal_add_balance(&referral_1, referral_1_fee);
                        if let Some(referral_2) = referral_2 {
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

                    if self.get_store_user_tokens(seller_id.clone()) {
                        // store a copy of a token to seller's collection
                    }

                    self.tokens.internal_transfer(
                        &seller_id,
                        &receiver_id,
                        &token_id,
                        None,
                        None,
                    );

                    token.clone()
                } else {
                    self.token_prices.insert(&token_id, &self.min_mint_price);
                    self.internal_mint_without_storage(token_id, receiver_id)
                }
            }
        }
    }
}

fn verification(pk_string: &[u8; 32], message: &str, sig_string: &[u8; 64]) -> bool {
    env::ed25519_verify(sig_string, message.as_bytes(), pk_string)
}
