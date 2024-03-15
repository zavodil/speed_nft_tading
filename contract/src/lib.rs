use std::borrow::BorrowMut;
use crate::utils::FeeFraction;
use near_contract_standards::fungible_token::Balance;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata,
};
use near_contract_standards::non_fungible_token::{
    NonFungibleToken, NonFungibleTokenEnumeration, Token, TokenId,
};
use near_sdk::{borsh::{BorshDeserialize, BorshSerialize}, collections::{LazyOption, UnorderedMap, UnorderedSet}, env, json_types::U128, near_bindgen, serde::{Deserialize, Serialize}, AccountId, BorshStorageKey, PanicOnDefault, PromiseOrValue, Timestamp, Gas, ext_contract, log};
use near_sdk::collections::Vector;
use near_sdk::store::{LookupMap, LookupSet};
use nft::*;

mod nft;
mod utils;
mod ft;

pub const TIMESTAMP_MAX_INTERVAL: u64 = 5 * 60 * 1_000_000_000;

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    NonFungibleToken,
    ContractMetadata,
    Enumeration,
    TokenMetadataTemplate,
    InternalBalances,
    StoreUserTokens,
    UserCollectionItems,
    UserCollectionItemsPerOwner { account_hash: Vec<u8> },
    TokenData
}

pub type TokenGeneration = u32; // ~ 4.3M resales
pub type StorageSize = u64;

#[derive(BorshDeserialize, BorshSerialize, Clone)]
#[borsh(crate = "near_sdk::borsh")]
struct TokenData {
    generation: TokenGeneration,
    price: Balance,
    last_sale: Option<Timestamp>,
}

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
struct CollectionItem {
    token_id: TokenId,
    generation: TokenGeneration,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    owner_id: AccountId,
    public_key: String,
    min_mint_price: Balance,
    // whitelisted token for deposits
    ft_account_id: AccountId,

    tokens: NonFungibleToken,
    contract_metadata: LazyOption<NFTContractMetadata>,
    token_metadata: LazyOption<TokenMetadata>,

    // referral rewards
    internal_balances: LookupMap<AccountId, Balance>,

    // shall we store user tokens
    is_store_user_tokens: LookupMap<AccountId, bool>,

    // generation, price, last_sale
    token_data: LookupMap<TokenId, TokenData>,

    // tokens in user collections
    //user_collection_items_1: LookupMap<AccountId, Vec<CollectionItem>>,
    user_collection_items: UnorderedMap<AccountId, UnorderedSet<CollectionItem>>,

    // fees
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
        seller_storage_size: StorageSize,
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
        ft_account_id: AccountId,
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
            ft_account_id: ft_account_id.clone(),
            public_key,
            min_mint_price: min_mint_price.0,
            tokens: nft_without_metadata(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::Enumeration),
            ),
            contract_metadata: LazyOption::new(
                StorageKey::ContractMetadata,
                Some(&contract_metadata),
            ),
            token_metadata: LazyOption::new(
                StorageKey::TokenMetadataTemplate,
                Some(&token_metadata),
            ),
            internal_balances: LookupMap::new(StorageKey::InternalBalances),
            is_store_user_tokens: LookupMap::new(StorageKey::StoreUserTokens),
            token_data: LookupMap::new(StorageKey::TokenData),
            user_collection_items: UnorderedMap::new(StorageKey::UserCollectionItems),
            mint_price_increase_fee,
            seller_fee,
            referral_1_fee,
            referral_2_fee,
        }
    }


}

impl Contract {
    pub(crate) fn get_new_token_data(&self) -> TokenData {
        TokenData {
            generation: 0u32,
            price: self.min_mint_price,
            last_sale: None
        }
    }

    pub(crate) fn get_token_data(&self, token_id: &TokenId) -> TokenData {
        self
            .token_data
            .get(token_id)
            .unwrap_or(&self.get_new_token_data())
            .clone()
    }

    pub(crate) fn get_token_price(&self, token_id: &TokenId) -> Balance{
        self.get_token_data(token_id).price
    }

    pub(crate) fn get_token_generation(&self, token_id: &TokenId) -> TokenGeneration {
        self.get_token_data(token_id).generation
    }

    pub(crate) fn get_user_collection(&self, account_id: &AccountId) -> Option<UnorderedSet<CollectionItem>> {
        self.user_collection_items.get(account_id)
    }

    pub(crate) fn get_user_collection_items(&self, account_id: &AccountId) -> StorageSize {
        if let Some(user_collection) =  self.user_collection_items.get(account_id) {
            user_collection.len()
        }
        else {
            0
        }
    }

    pub(crate) fn store_nft(&mut self, token_id: &TokenId, generation: TokenGeneration, account_id: &AccountId) {
        // save nft
        self.internal_mint_without_storage(generate_token_id(&generation, token_id), account_id.clone());


        // add to user collection
        let mut user_collection = if let Some(user_collection) =  self.user_collection_items.get(account_id) {
            user_collection
        }
        else {
            UnorderedSet::new(
                StorageKey::UserCollectionItemsPerOwner {
                    account_hash: env::sha256(account_id.as_bytes()),
                },
            )
        };

        user_collection.insert(&CollectionItem {token_id: token_id.clone(), generation});
        self.user_collection_items.insert(account_id, &user_collection);
    }

    /* message - a stringified JSON Object
   {
       "token_id": "<ipfs_hash>",
       "account_id": "buyer_name.near",
       "seller_storage_size": 3,
       "referral_id_1": "ref.near",
       "referral_id_2": "ref.near",
       timestamp: Timestamp
   }
    */
    // signature - message signed with self.public_key

    pub(crate) fn nft_mint(&mut self, message: String, signature: String, receiver_id: AccountId, deposit: Balance) -> Token {
        let mut pk = [0u8; 32];
        let _pk_bytes = hex::decode_to_slice(self.public_key.clone(), &mut pk as &mut [u8]);

        let mut sig = [0u8; 64];
        let _signature = hex::decode_to_slice(signature, &mut sig as &mut [u8]);

        assert!(verification(&pk, &message, &sig), "Signature check failed");

        let parsed_message = serde_json::from_str::<MintNftMsg>(&message).expect("Wrong message format");

        match parsed_message {
            MintNftMsg::SimpleMint {
                token_id, account_id, referral_1, referral_2, timestamp, seller_storage_size
            } => {
                assert_eq!(receiver_id, account_id, "Mint for yourself only");

                assert!(
                    timestamp + TIMESTAMP_MAX_INTERVAL >= env::block_timestamp(),
                    "Timestamp is too old"
                );

                if let Some(token) = self.tokens.nft_token(token_id.clone()) {
                    // token already exists
                    let token_data: TokenData = self.get_token_data(&token_id);
                    let old_price: Balance = token_data.price;
                    let old_generation: TokenGeneration = token_data.generation;

                    if let Some(token_last_sale) = token_data.last_sale {
                        assert!(
                            timestamp >= token_last_sale,
                            "Timestamp is older then last sale"
                        );
                    }

                    let price_increase = self.mint_price_increase_fee.multiply(old_price);
                    let new_price = old_price + price_increase;

                    assert_deposit(deposit, new_price);

                    // distribute seller reward
                    let seller_id: AccountId = token.owner_id.clone();
                    assert_ne!(seller_id, receiver_id, "Current and next owner must differ");

                    let seller_fee: Balance = self.seller_fee.multiply(price_increase);
                    // TODO ft transfer here instead
                    // self.internal_add_balance(&seller_id, old_price + seller_fee);

                    // store old token
                    if self.get_store_user_tokens(seller_id.clone()) && seller_storage_size > self.get_user_collection_items(&seller_id) {
                        log!("store_nft {}:{}", token_id.clone(), old_generation.clone());
                        self.store_nft(&token_id, old_generation, &seller_id)
                    }

                    // update token data
                    self.token_data.insert(token_id.clone(),
                                           TokenData { generation: old_generation + 1, price: new_price, last_sale: Some(env::block_timestamp())});

                    // distribute affiliate reward
                    let mut referral_1_fee: Balance = 0;
                    let mut referral_2_fee: Balance = 0;
                    if let Some(referral_1) = referral_1 {
                        referral_1_fee = self.referral_1_fee.multiply(price_increase);
                        self.internal_add_balance(&referral_1, referral_1_fee);
                        if let Some(referral_2) = referral_2 {
                            referral_2_fee = self.referral_2_fee.multiply(price_increase);
                            self.internal_add_balance(&referral_2, referral_2_fee);
                        }
                    }

                    // distribute system reward
                    let mut system_fee = Some(price_increase);
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
                    // create new token
                    let old_price = self.min_mint_price;

                    assert_deposit(deposit, old_price);

                    self.token_data.insert(token_id.clone(),
                                           TokenData { generation: 0, price: old_price, last_sale: Some(env::block_timestamp())});
                    self.internal_mint_without_storage(token_id, receiver_id)
                }
            }
        }
    }

}

fn assert_deposit(deposit: Balance, price: Balance) {
    assert!(deposit >= price, "Illegal deposit, add extra {} yNEAR", price - deposit);
}

fn verification(pk_string: &[u8; 32], message: &str, sig_string: &[u8; 64]) -> bool {
    env::ed25519_verify(sig_string, message.as_bytes(), pk_string)
}
