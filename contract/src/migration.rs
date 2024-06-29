use crate::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[allow(dead_code)]
    #[private]
    pub fn migrate_1(max_storage_size: StorageSize, user_mint_price: U128) -> Self {
        #[derive(BorshDeserialize)]
        #[borsh(crate = "near_sdk::borsh")]
        struct OldContract {
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

            // timestamp of the last purchase to avoid double usage of the signature
            last_user_action: LookupMap<AccountId, Timestamp>,

            // tokens in user collections
            //user_collection_items_1: LookupMap<AccountId, Vec<CollectionItem>>,
            user_collection_items: UnorderedMap<AccountId, UnorderedSet<CollectionItem>>,

            // fees
            mint_price_increase_fee: FeeFraction,
            seller_fee: FeeFraction,
            referral_1_fee: FeeFraction,
            referral_2_fee: FeeFraction,
        }

        let old_contract: OldContract = env::state_read().expect("Old state doesn't exist");

        Self {
            owner_id: old_contract.owner_id,
            public_key: old_contract.public_key,
            min_mint_price: old_contract.min_mint_price,
            ft_account_id: old_contract.ft_account_id,
            tokens: old_contract.tokens,
            contract_metadata: old_contract.contract_metadata,
            token_metadata: old_contract.token_metadata,
            internal_balances: old_contract.internal_balances,
            is_store_user_tokens: old_contract.is_store_user_tokens,
            token_data: old_contract.token_data,
            last_user_action: old_contract.last_user_action,
            user_collection_items: old_contract.user_collection_items,
            mint_price_increase_fee: old_contract.mint_price_increase_fee,
            seller_fee: old_contract.seller_fee,
            referral_1_fee: old_contract.referral_1_fee,
            referral_2_fee: old_contract.referral_2_fee,

            storage: LookupMap::new(StorageKey::Storage),
            max_storage_size,
            storage_packages: UnorderedMap::new(StorageKey::StoragePackages),

            user_token_requests: UnorderedMap::new(StorageKey::UserTokens),
            last_user_token_request_id: 0,
            user_mint_price: user_mint_price.0,
            storage_contract_id: None
        }
    }

    #[init(ignore_state)]
    #[allow(dead_code)]
    #[private]
    pub fn migrate_2(user_mint_price: U128) -> Self {
        #[derive(BorshDeserialize)]
        #[borsh(crate = "near_sdk::borsh")]
        struct OldContract {
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
            storage_packages: UnorderedMap<StoragePackageIndex, StoragePackage>,
        }

        let old_contract: OldContract = env::state_read().expect("Old state doesn't exist");

        Self {
            owner_id: old_contract.owner_id,
            public_key: old_contract.public_key,
            min_mint_price: old_contract.min_mint_price,
            ft_account_id: old_contract.ft_account_id,
            tokens: old_contract.tokens,
            contract_metadata: old_contract.contract_metadata,
            token_metadata: old_contract.token_metadata,
            internal_balances: old_contract.internal_balances,
            is_store_user_tokens: old_contract.is_store_user_tokens,
            token_data: old_contract.token_data,
            last_user_action: old_contract.last_user_action,
            user_collection_items: old_contract.user_collection_items,
            mint_price_increase_fee: old_contract.mint_price_increase_fee,
            seller_fee: old_contract.seller_fee,
            referral_1_fee: old_contract.referral_1_fee,
            referral_2_fee: old_contract.referral_2_fee,

            storage: old_contract.storage,
            max_storage_size: old_contract.max_storage_size,
            storage_packages: old_contract.storage_packages,

            user_token_requests: UnorderedMap::new(StorageKey::UserTokens),
            last_user_token_request_id: 0,
            user_mint_price: user_mint_price.0,
            storage_contract_id: None
        }
    }
}