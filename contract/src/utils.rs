use crate::*;

#[near_bindgen]
impl Contract {
    pub fn get_ft_account_id(&self) -> AccountId {
        self.ft_account_id.clone()
    }

    pub fn get_collection_items(&self, account_id: AccountId) -> StorageSize {
        self.get_user_collection_items(&account_id)
    }

    pub fn get_collection(&self, account_id: AccountId) -> Option<Vec<CollectionItem>> {
        if let Some(collection) = self.get_user_collection(&account_id){
            Some(collection.to_vec())
        }
        else {
            None
        }
    }

    pub fn set_store_user_tokens(&mut self, store_tokens: bool) {
        self.is_store_user_tokens.insert(env::predecessor_account_id(), store_tokens);
    }

    pub fn get_store_user_tokens(&self, account_id: AccountId) -> bool {
        self.is_store_user_tokens.get(&account_id).unwrap_or(&false).clone()
    }

    pub fn set_min_mint_price(&mut self, min_mint_price: U128) {
        self.assert_owner();
        self.min_mint_price = min_mint_price.0;
    }

    pub fn get_min_mint_price(&self) -> U128{
        U128::from(self.min_mint_price)
    }

    pub fn get_public_key(&mut self) -> String {
        self.public_key.clone()
    }

    pub fn set_public_key(&mut self, public_key: String) {
        self.assert_owner();
        self.public_key = public_key;
    }

    pub fn set_mint_price_increase_fee(&mut self, mint_price_increase_fee: FeeFraction) {
        self.assert_owner();
        self.mint_price_increase_fee = mint_price_increase_fee;
    }

    pub fn set_seller_fee(&mut self, seller_fee: FeeFraction) {
        self.assert_owner();
        self.seller_fee = seller_fee;
    }

    pub fn set_referral_fee(&mut self, referral_1_fee: FeeFraction, referral_2_fee: FeeFraction) {
        self.assert_owner();
        self.referral_1_fee = referral_1_fee;
        self.referral_2_fee = referral_2_fee;
    }

    pub fn get_balance(&self, account_id: AccountId) -> U128 {
        U128::from(self.internal_balances.get(&account_id).unwrap_or(&0u128).clone())
    }

    pub fn remove_user_collection_item(&mut self, generation: TokenGeneration, token_id: TokenId) {
        let account_id = env::predecessor_account_id();

        let mut user_collection = self.user_collection_items.get(&account_id).expect("Not found");

        let item_to_remove = CollectionItem {token_id: token_id.clone(), generation};
        if user_collection.contains(&item_to_remove) {
            log!("Item removed: {}:{}", generation, token_id.clone());

            user_collection.remove(&item_to_remove);
            self.user_collection_items.insert(&account_id, &user_collection);
        }
        else {
            panic!("Not found");
        }

        // remove NFT
        let full_token_id = generate_token_id(&generation, &token_id);
        self.tokens.owner_by_id.remove(&full_token_id);

        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let mut token_ids = tokens_per_owner.get(&account_id).expect("Not found");
            token_ids.remove(&token_id);
            tokens_per_owner.insert(&account_id, &token_ids);
        }
    }

    // returns [token, [generation, price]]
    pub fn get_token(&self, token_id: TokenId) -> (Option<Token>, Option<(TokenGeneration, U128)>) {
        if let Some(token) = self.tokens.nft_token(token_id.clone()) {

            // token from user collection
            if token_id.contains(':') {
                return (Some(token), None);
            }

            let token_data = self.get_token_data(&token_id);
            (
                Some(token),
                Some((token_data.generation, U128::from(token_data.price)))
            )
        } else {
            (None, None)
        }
    }

    // returns [token, next_price, seller_collection_items, seller_total_items, seller_is_store_tokens]
    pub fn get_token_for_sale(&self, token_id: TokenId) -> Option<(Token, U128, StorageSize, StorageSize, bool)> {
        // token from user collection
        if token_id.contains(':') {
            return None;
        }

        if let Some(token) = self.tokens.nft_token(token_id.clone()) {
            let old_price: Balance = self.get_token_price(&token_id);

            let price_increase = self.mint_price_increase_fee.multiply(old_price);

            let seller_collection_items =  self.get_user_collection_items(&token.owner_id);
            let seller_total_items = self.internal_total_supply_by_user(&token.owner_id);
            let seller_is_store_tokens = *self.is_store_user_tokens.get(&token.owner_id).unwrap_or(&false);

            Some((token, U128::from(old_price + price_increase), seller_collection_items, seller_total_items, seller_is_store_tokens))
        } else {
            None
        }
    }
}

impl Contract {
    pub fn assert_owner(&self) {
        assert_eq!(self.owner_id, env::predecessor_account_id(), "Not an owner");
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct FeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

impl FeeFraction {
    pub fn assert_valid(&self) {
        assert_ne!(self.denominator, 0, "Denominator must be a positive number");
        assert!(
            self.numerator <= self.denominator,
            "The treasure fee must be less or equal to 1"
        );
    }

    pub fn multiply(&self, value: Balance) -> Balance {
        (U256::from(self.numerator) * U256::from(value) / U256::from(self.denominator)).as_u128()
    }
}

use uint::construct_uint;
construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}
