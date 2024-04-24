use crate::utils::{assert_fees_overflow, FeeFraction};
use near_contract_standards::fungible_token::Balance;
use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata,
};
use near_contract_standards::non_fungible_token::{
    NonFungibleToken, NonFungibleTokenEnumeration, Token, TokenId,
};
use near_sdk::{borsh::{BorshDeserialize, BorshSerialize}, collections::{LazyOption, UnorderedMap, UnorderedSet}, NearToken, env, json_types::U128, Promise, near_bindgen, serde::{Deserialize, Serialize}, AccountId, BorshStorageKey, PanicOnDefault, PromiseOrValue, Timestamp, Gas, ext_contract, log};
use near_sdk::store::{LookupMap};
use nft::{nft_without_metadata, generate_token_id};

mod nft;
mod utils;
mod ft;
mod account;
mod market;
mod events;
mod migration;

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
    TokenData,
    LastUserAction,
    Storage,
    StoragePackages,
}

pub type TokenGeneration = u32; // ~ 4.3M resales
pub type StorageSize = u64;
pub type StoragePackageIndex = u64;

#[derive(BorshDeserialize, BorshSerialize, Clone)]
#[borsh(crate = "near_sdk::borsh")]
struct TokenData {
    generation: TokenGeneration,
    price: Balance
}

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Clone, Serialize)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct CollectionItem {
    token_id: TokenId,
    generation: TokenGeneration,
}

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Clone, Deserialize, Serialize)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
struct StoragePackage {
    storage_size: StorageSize,
    price: Balance,
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

    // referral rewards + refunds for fallen withdrawals
    internal_balances: LookupMap<AccountId, Balance>,

    // shall we store user tokens in user_collection
    is_store_user_tokens: LookupMap<AccountId, bool>,

    // generation, price, last_sale
    token_data: LookupMap<TokenId, TokenData>,

    // timestamp of the last purchase to avoid double usage of the signature
    last_user_action: LookupMap<AccountId, Timestamp>,

    // tokens in user collections
    user_collection_items: UnorderedMap<AccountId, UnorderedSet<CollectionItem>>,

    // fees
    mint_price_increase_fee: FeeFraction,
    seller_fee: FeeFraction,
    referral_1_fee: FeeFraction,
    referral_2_fee: FeeFraction,

    // storage
    storage: LookupMap<AccountId, StorageSize>,
    max_storage_size: StorageSize,
    storage_packages: UnorderedMap<StoragePackageIndex, StoragePackage>
}

#[derive(Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, Serialize))]
#[serde(crate = "near_sdk::serde")]
pub enum MintNftMsg {
    SimpleMint {
        token_id: TokenId,
        account_id: AccountId,
        referral_id_1: Option<AccountId>,
        referral_id_2: Option<AccountId>,
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
        max_storage_size: StorageSize
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        assert_fees_overflow(vec![&seller_fee, &referral_1_fee, &referral_2_fee]);

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
            last_user_action: LookupMap::new(StorageKey::LastUserAction),
            user_collection_items: UnorderedMap::new(StorageKey::UserCollectionItems),
            mint_price_increase_fee,
            seller_fee,
            referral_1_fee,
            referral_2_fee,

            storage: LookupMap::new(StorageKey::Storage),
            max_storage_size,
            storage_packages: UnorderedMap::new(StorageKey::StoragePackages)
        }
    }


}

