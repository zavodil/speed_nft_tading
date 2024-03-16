use crate::*;
use near_sdk::collections::{LookupMap, TreeMap, UnorderedSet};
use near_sdk::{require, IntoStorageKey};

impl Contract {
    pub fn get_token_metadata(&self, token_id: &TokenId) -> TokenMetadata {
        let mut token_metadata = self.token_metadata.get().unwrap();
        token_metadata.media = Some(format!("ipfs://{}", token_id));

        token_metadata
    }

    fn enum_get_token(&self, owner_id: AccountId, token_id: TokenId) -> Token {
        let metadata = self.get_token_metadata(&token_id);
        let approved_account_ids = self.tokens.approvals_by_id.as_ref().map(|approvals_by_id| {
            approvals_by_id
                .get(&token_id.to_string())
                .unwrap_or_default()
        });

        Token {
            token_id,
            owner_id,
            metadata: Some(metadata),
            approved_account_ids,
        }
    }

    pub(crate) fn internal_mint_without_storage(
        &mut self,
        token_id: TokenId,
        token_owner_id: AccountId,
    ) -> Token {
        if self.tokens.owner_by_id.get(&token_id).is_some() {
            env::panic_str("token_id must be unique");
        }

        let owner_id: AccountId = token_owner_id;

        // Core behavior: every token must have an owner
        self.tokens.owner_by_id.insert(&token_id, &owner_id);

        // Enumeration extension: Record tokens_per_owner for use with enumeration view methods.
        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let mut token_ids = tokens_per_owner.get(&owner_id).unwrap_or_else(|| {
                UnorderedSet::new(
                    near_contract_standards::non_fungible_token::core::StorageKey::TokensPerOwner {
                        account_hash: env::sha256(owner_id.as_bytes()),
                    },
                )
            });
            token_ids.insert(&token_id);
            tokens_per_owner.insert(&owner_id, &token_ids);
        }

        Token {
            token_id,
            owner_id,
            metadata: None,
            approved_account_ids: None,
        }
    }
}

pub(crate) fn nft_without_metadata<Q, S>(
    owner_by_id_prefix: Q,
    owner_id: AccountId,
    enumeration_prefix: Option<S>
) -> NonFungibleToken
where
    Q: IntoStorageKey,
    S: IntoStorageKey
{
    NonFungibleToken {
        owner_id,
        extra_storage_in_bytes_per_token: 0,
        owner_by_id: TreeMap::new(owner_by_id_prefix),
        token_metadata_by_id: None,
        tokens_per_owner: enumeration_prefix.map(LookupMap::new),
        approvals_by_id: None,
        next_approval_id_by_id: None,
    }

    // removed since extra_storage_in_bytes_per_token is not used anywhere in the nft standard
    // this.measure_min_token_storage_cost();
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
        let owner_id = self.tokens.owner_by_id.get(&token_id).expect("Not found");
        let metadata = self.get_token_metadata(&token_id);

        Some(Token {
            token_id,
            owner_id,
            metadata: Some(metadata),
            approved_account_ids: None,
        })
}
}

#[near_bindgen]
impl NonFungibleTokenEnumeration for Contract {
    fn nft_total_supply(&self) -> U128 {
        self.tokens.nft_total_supply()
    }

    fn nft_tokens(
        &self,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        // Get starting index, whether or not it was explicitly given.
        // Defaults to 0 based on the spec:
        // https://nomicon.io/Standards/NonFungibleToken/Enumeration.html#interface
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        require!(
            (self.tokens.owner_by_id.len() as u128) >= start_index,
            "Out of bounds, please use a smaller from_index."
        );
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "Cannot provide limit of 0.");
        self.tokens
            .owner_by_id
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|(token_id, owner_id)| self.enum_get_token(owner_id, token_id))
            .collect()
    }

    fn nft_supply_for_owner(&self, account_id: AccountId) -> near_sdk::json_types::U128 {
        self.tokens.nft_supply_for_owner(account_id)
    }

    fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        let tokens_per_owner = self.tokens.tokens_per_owner.as_ref().unwrap_or_else(|| {
            env::panic_str(
                "Could not find tokens_per_owner when calling a method on the \
                enumeration standard.",
            )
        });
        let token_set = if let Some(token_set) = tokens_per_owner.get(&account_id) {
            token_set
        } else {
            return vec![];
        };

        if token_set.is_empty() {
            return vec![];
        }

        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "Cannot provide limit of 0.");
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        require!(
            token_set.len() as u128 > start_index,
            "Out of bounds, please use a smaller from_index."
        );
        token_set
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|token_id| self.enum_get_token(account_id.clone(), token_id))
            .collect()
    }
}

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.contract_metadata.get().unwrap()
    }
}

pub fn parse_token_id (token_id: TokenId) -> (TokenGeneration, TokenId) {
    let parts: Vec<&str> = token_id.split(':').collect();

    if parts.len() >= 2 {

        let generation: TokenGeneration = parts[0].parse().unwrap_or_else(|_| { 0 });
        let token_id = parts[1];

        (generation, token_id.to_string())
    } else {
        (0, token_id)
    }
}

pub fn generate_token_id (generation: &TokenGeneration, token_id: &TokenId) -> TokenId {
    format!("{}:{}", generation, token_id)
}