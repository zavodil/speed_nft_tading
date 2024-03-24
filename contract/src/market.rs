use crate::*;
use crate::ft::{GAS_FOR_AFTER_FT_TRANSFER, GAS_FOR_FT_TRANSFER};

const GAS_FOR_RESALE: Gas = Gas::from_tgas(GAS_FOR_AFTER_FT_TRANSFER.as_tgas() + GAS_FOR_FT_TRANSFER.as_tgas() + 15u64);

impl Contract {
    pub(crate) fn get_new_token_data(&self) -> TokenData {
        TokenData {
            generation: 0u32,
            price: self.min_mint_price
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
    },
    signature - message signed with self.public_key

    This function doesn't check if buyer has enough storage to keep the token. We expect server to make this check before to verify the transaction.
    */


    pub(crate) fn nft_mint(&mut self, message: String, signature: String, receiver_id: AccountId, deposit: Balance) -> PromiseOrValue<bool> {
        let mut pk = [0u8; 32];
        let _pk_bytes = hex::decode_to_slice(self.public_key.clone(), &mut pk as &mut [u8]);

        let mut sig = [0u8; 64];
        let _signature = hex::decode_to_slice(signature, &mut sig as &mut [u8]);

        assert!(verification(&pk, &message, &sig), "Signature check failed");

        let parsed_message = serde_json::from_str::<MintNftMsg>(&message).expect("Wrong message format");

        match parsed_message {
            MintNftMsg::SimpleMint {
                token_id, account_id, referral_id_1, referral_id_2, timestamp, seller_storage_size
            } => {
                assert_eq!(receiver_id, account_id, "Mint for yourself only");

                assert!(
                    timestamp + TIMESTAMP_MAX_INTERVAL >= env::block_timestamp(),
                    "Timestamp is too old"
                );

                if let Some(user_last_action) = self.last_user_action.get(&account_id) {
                    assert!(
                        timestamp > *user_last_action,
                        "Timestamp is smaller then last user's action"
                    );
                }

                // save buyer's action
                self.last_user_action.insert(account_id, env::block_timestamp());

                if let Some(token) = self.tokens.nft_token(token_id.clone()) {
                    // token already exists
                    assert!(remaining_gas() >= GAS_FOR_RESALE, "Attach more gas");

                    let token_data: TokenData = self.get_token_data(&token_id);
                    let old_price: Balance = token_data.price;
                    let old_generation: TokenGeneration = token_data.generation;

                    let price_increase = self.mint_price_increase_fee.multiply(old_price);
                    let new_price = old_price + price_increase;

                    assert_deposit(deposit, new_price);

                    // distribute seller reward
                    let seller_id: AccountId = token.owner_id.clone();
                    assert_ne!(seller_id, receiver_id, "Current and next owner must differ");

                    // store old token
                    if self.get_store_user_tokens(seller_id.clone()) && seller_storage_size > self.internal_total_supply_by_user(&seller_id) {
                        log!("store_nft {}:{}", token_id.clone(), old_generation.clone());
                        self.store_nft(&token_id, old_generation, &seller_id)
                    }

                    // update token data
                    self.token_data.insert(token_id.clone(),
                                           TokenData { generation: old_generation + 1, price: new_price});

                    // fees on nft price increase
                    let seller_fee = self.manage_fees(false, &token_id, &receiver_id, price_increase, referral_id_1, referral_id_2);

                    self.tokens.internal_transfer(
                        &seller_id,
                        &receiver_id,
                        &token_id,
                        None,
                        None,
                    );

                    let seller_payout = old_price + seller_fee;
                    events::emit::add_seller_payout(&receiver_id, &token_id, seller_payout);

                    // ft transfer to seller here instead
                    PromiseOrValue::Promise(self.internal_ft_transfer(&seller_id, seller_payout))

                } else {
                    // create new token
                    let min_price = self.min_mint_price;

                    assert_deposit(deposit, min_price);

                    // fees on initial payment
                    self.manage_fees(true, &token_id, &receiver_id, min_price, referral_id_1, referral_id_2);

                    self.token_data.insert(token_id.clone(), TokenData { generation: 0, price: min_price });
                    self.internal_mint_without_storage(token_id, receiver_id);

                    PromiseOrValue::Value(true)
                }
            }
        }
    }

    // returns seller fee
    pub(crate) fn manage_fees (&mut self, initial_sale: bool, token_id: &TokenId, account_id: &AccountId, price_increase: Balance, referral_id_1: Option<AccountId>, referral_id_2: Option<AccountId>) -> Balance {
        let seller_fee: Balance = if !initial_sale { self.seller_fee.multiply(price_increase) } else { 0 };

        // distribute affiliate reward
        let mut referral_1_fee: Balance = 0;
        let mut referral_2_fee: Balance = 0;
        if let Some(referral_1) = referral_id_1 {
            referral_1_fee = self.referral_1_fee.multiply(price_increase);
            events::emit::add_referral_fee(&referral_1, account_id, token_id, referral_1_fee);
            self.internal_add_balance(&referral_1, referral_1_fee);
        }
        if let Some(referral_2) = referral_id_2 {
            referral_2_fee = self.referral_2_fee.multiply(price_increase);
            events::emit::add_referral_fee(&referral_2, account_id, token_id, referral_2_fee);
            self.internal_add_balance(&referral_2, referral_2_fee);
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
            events::emit::add_system_fee(&self.owner_id, token_id, system_fee);
            self.internal_add_balance(&self.owner_id.clone(), system_fee);
        }

        seller_fee
    }
}

fn assert_deposit(deposit: Balance, price: Balance) {
    assert!(deposit >= price, "Illegal deposit, add extra {} yNEAR", price - deposit);
}

fn verification(pk_string: &[u8; 32], message: &str, sig_string: &[u8; 64]) -> bool {
    env::ed25519_verify(sig_string, message.as_bytes(), pk_string)
}

fn remaining_gas() -> Gas {
    Gas::from_gas(env::prepaid_gas().as_gas() - env::used_gas().as_gas())
}
