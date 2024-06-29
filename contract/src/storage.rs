use near_sdk::PromiseResult;
use crate::*;
use crate::ft::{ext_market_contract, ext_self};

pub const GAS_FOR_STORAGE_TRANSFER: Gas = Gas::from_tgas(20);
pub const GAS_FOR_STORAGE_TRANSFER_CALLBACK: Gas = Gas::from_tgas(20);

impl Contract {
    pub(crate) fn buy_storage(&mut self, receiver_id: AccountId, deposit: Balance, index: StoragePackageIndex) {
        let package = self.storage_packages.get(&index).expect("Missing Storage Package");
        assert!(deposit >= package.price , "Illegal Deposit");

        let old_storage = self.internal_get_user_storage(&receiver_id);
        let new_storage = old_storage + package.storage_size;
        assert!(new_storage <= self.max_storage_size, "Illegal Storage To Buy");

        self.storage.insert(receiver_id, new_storage);
    }
}

#[near_bindgen]
impl Contract {
    pub fn transfer_storage(&mut self, receiver_contract_id: AccountId) -> Promise {
        let account_id = env::predecessor_account_id();

        let storage_size = self.internal_get_user_storage(&account_id);
        let storage_used = self.internal_total_supply_by_user(&account_id);
        assert!(storage_size > storage_used, "Illegal storage");

        let user_storage = storage_size - storage_used;
        assert!(user_storage > 0, "Nothing to transfer");

        self.storage.insert(account_id.clone(), 0);

        ext_market_contract::ext(receiver_contract_id)
            .with_static_gas(GAS_FOR_STORAGE_TRANSFER)
            .on_transfer_storage(account_id.clone(), user_storage.clone())
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_STORAGE_TRANSFER_CALLBACK)
                    .callback_after_transfer_storage(account_id, user_storage)
            )
    }

    pub fn on_transfer_storage(&mut self, account_id: AccountId, user_storage: StorageSize) {
        assert_eq!(env::signer_account_id(), self.storage_contract_id.clone().expect("Storage Transfer Disabled"));

        let old_storage = self.internal_get_user_storage(&account_id);
        let new_storage = old_storage + user_storage;
        assert!(new_storage <= self.max_storage_size, "Illegal Storage Value");

        events::emit::storage_transferred(&account_id, user_storage);

        self.storage.insert(account_id, new_storage);
    }

    #[private]
    pub fn callback_after_transfer_storage(&mut self, account_id: AccountId, user_storage: StorageSize) {
        assert_eq!(env::promise_results_count(), 1, "Err: expected 1 promise result from withdraw");
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                events::emit::transfer_storage_succeeded(&account_id, user_storage);
            }
            PromiseResult::Failed => {
                events::emit::transfer_storage_failed(&account_id, user_storage);

                let old_storage = self.internal_get_user_storage(&account_id);
                let new_storage = old_storage + user_storage;
                assert!(new_storage <= self.max_storage_size, "Illegal Storage Value");

                self.storage.insert(account_id, new_storage);
            }
        };
    }
}