use crate::*;
use crate::utils::unordered_map_pagination;

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum TokenStatus {
    PENDING,
    APPROVED,
    REJECTED
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct UserToken {
    pub token_id: TokenId,
    pub account_id: AccountId,
    pub status: TokenStatus,
    pub created_at: Timestamp
}

impl Contract {
    pub(crate) fn mint_user_token(&mut self, token_id: TokenId, account_id: AccountId) {
        let token = UserToken {
            token_id,
            account_id,
            status: TokenStatus::PENDING,
            created_at: env::block_timestamp()
        };

        self.last_user_token_request_id += 1;
        self.user_token_requests.insert(&self.last_user_token_request_id, &token);
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_user_mint_requests(&self, from_index: Option<u64>,
                                  limit: Option<u64>) -> Vec<(u64, UserToken)> {
        return unordered_map_pagination(&self.user_token_requests, from_index, limit);
    }
}