use crate::*;

impl Contract {
    pub(crate) fn internal_add_balance(&mut self, account_id: &AccountId, value: Balance) {
        if value > 0 {
            let prev_balance: Balance = self.internal_balances.get(account_id).unwrap_or(&0u128).clone();
            self.internal_balances.insert(account_id.clone(), prev_balance + value);
        }
    }
}

#[near_bindgen]
impl Contract{
    pub fn withdraw(&mut self, amount: Option<U128>) -> Promise {
        let account_id = env::predecessor_account_id();
        let balance: Balance = self.internal_balances.get(&account_id).unwrap_or(&0u128).clone();

        let amount: Balance = if let Some(amount) = amount {
            assert!(balance >= amount.0, "Balance is too small");
            amount.0
        } else {
            balance
        };

        assert!(amount > 0, "Positive amount required");

        self.internal_balances
            .insert(account_id.clone(), balance - amount);

        self.internal_ft_transfer(&account_id, amount)
    }

}
