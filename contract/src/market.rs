use crate::*;
use crate::ft::{GAS_FOR_AFTER_FT_TRANSFER, GAS_FOR_FT_TRANSFER};

const GAS_FOR_RESALE: Gas = Gas::from_tgas(GAS_FOR_AFTER_FT_TRANSFER.as_tgas() + GAS_FOR_FT_TRANSFER.as_tgas() + 15u64);

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
    },
    signature - message signed with self.public_key

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

                if let Some(token) = self.tokens.nft_token(token_id.clone()) {
                    // token already exists
                    assert!(remaining_gas() >= GAS_FOR_RESALE, "Attach more gas");

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
                    if let Some(referral_1) = referral_id_1 {
                        referral_1_fee = self.referral_1_fee.multiply(price_increase);
                        self.internal_add_balance(&referral_1, referral_1_fee);
                    }
                    if let Some(referral_2) = referral_id_2 {
                        referral_2_fee = self.referral_2_fee.multiply(price_increase);
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

                    // ft transfer to seller here instead
                    PromiseOrValue::Promise(self.internal_ft_transfer(&seller_id, old_price + seller_fee))

                } else {
                    // create new token
                    let old_price = self.min_mint_price;

                    assert_deposit(deposit, old_price);

                    self.token_data.insert(token_id.clone(),
                                           TokenData { generation: 0, price: old_price, last_sale: Some(env::block_timestamp())});
                    self.internal_mint_without_storage(token_id, receiver_id);

                    PromiseOrValue::Value(true)
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

fn remaining_gas() -> Gas {
    Gas::from_gas(env::prepaid_gas().as_gas() - env::used_gas().as_gas())
}
