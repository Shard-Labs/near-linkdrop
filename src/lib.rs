use borsh::{BorshDeserialize, BorshSerialize};
use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::collections::LookupSet;
use near_sdk::json_types::ValidAccountId;
use near_sdk::json_types::{Base58PublicKey, U128};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Gas, Promise, PromiseResult, PublicKey,
};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct LinkDrop {
    // Lookup map that indicates which accounts are eligible to claim the nft
    pub accounts: LookupSet<PublicKey>,
    // Stores the nft info to be claimed
    pub nft_contract_id: AccountId,
}

/// Access key allowance for linkdrop keys.
const ACCESS_KEY_ALLOWANCE: u128 = 1_000_000_000_000_000_000_000_000;

/// Gas to spend for nft transaction
const TRANSFER_FROM_GAS: Gas = 10_000_000_000_000;

/// Gas attached to the callback from account creation.
pub const ON_CREATE_ACCOUNT_CALLBACK_GAS: u64 = 10_000_000_000_000;

/// Indicates there are no deposit for a callback for better readability.
const NO_DEPOSIT: u128 = 0;

#[ext_contract(ext_nft)]
pub trait ExtNFTContract {
    fn nft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) -> Promise;
}

#[ext_contract(ext_self)]
pub trait ExtLinkDrop {
    /// Callback after plain account creation.
    fn on_account_created(&mut self, predecessor_account_id: AccountId, amount: U128) -> bool;

    /// Callback after creating account and claiming linkdrop.
    fn on_account_created_and_claimed(&mut self) -> bool;

    /// Callback to update the nft_contract_id
    fn update_nft_storage(&mut self, public_key: PublicKey);
}

fn is_promise_success() -> bool {
    assert_eq!(
        env::promise_results_count(),
        1,
        "Contract expected a result on the callback"
    );
    match env::promise_result(0) {
        PromiseResult::Successful(_) => true,
        _ => false,
    }
}

impl Default for LinkDrop {
    fn default() -> Self {
        let empty_set: LookupSet<PublicKey> = LookupSet::new(0);
        Self {
            accounts: empty_set,
            nft_contract_id: String::from(""),
        }
    }
}

#[near_bindgen]
impl LinkDrop {
    #[init]
    pub fn init(nft_contract_id: AccountId) -> Self {
        Self {
            accounts: LookupSet::new(0),
            nft_contract_id: nft_contract_id,
        }
    }

    /// Allows given public key to claim sent balance.
    /// Takes ACCESS_KEY_ALLOWANCE as fee from deposit to cover account creation via an access key.
    #[payable]
    pub fn send(&mut self, public_key: Base58PublicKey) -> Promise {
        let pk = public_key.into();
        let new_account = self.accounts.insert(&pk);
        // If the set did not have this value present, true is returned
        assert!(new_account, "Account already registered");
        Promise::new(env::current_account_id()).add_access_key(
            pk,
            ACCESS_KEY_ALLOWANCE,
            env::current_account_id(),
            // add_access_key allows given pk to call functions claim or create_account_and_claim
            b"claim,create_account_and_claim".to_vec(),
        )
    }

    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: ValidAccountId, token_id: TokenId) -> Promise {
        assert!(
            self.accounts.contains(&env::signer_account_pk()),
            "Signer must be eligible to claim the NFT"
        );
        Promise::new(env::current_account_id()).then(
            ext_nft::nft_transfer(
                account_id.clone(),
                token_id,
                None,
                None, //Memo
                &self.nft_contract_id.clone(),
                1,
                TRANSFER_FROM_GAS,
            )
            .then(ext_self::update_nft_storage(
                env::signer_account_pk(),
                &env::current_account_id(),
                NO_DEPOSIT,
                TRANSFER_FROM_GAS,
            )),
        )
    }

    /// Create new account and and claim tokens to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: ValidAccountId,
        new_public_key: Base58PublicKey,
        token_id: TokenId,
    ) -> Promise {
        assert!(
            self.accounts.contains(&env::signer_account_pk()),
            "Signer must be eligible to claim the NFT"
        );
        // Check if pk is in accounts lookupset
        if self.accounts.contains(&env::signer_account_pk()) {
            Promise::new(new_account_id.to_string())
                .create_account()
                .add_full_access_key(new_public_key.clone().into()) // TODO: Check if this is necessary
                .then(ext_nft::nft_transfer(
                    new_account_id,
                    token_id,
                    None,
                    None,
                    &self.nft_contract_id,
                    1,
                    TRANSFER_FROM_GAS,
                ))
                .then(ext_self::update_nft_storage(
                    new_public_key.into(),
                    &env::current_account_id(),
                    NO_DEPOSIT,
                    TRANSFER_FROM_GAS,
                ))
        } else {
            panic!("Public key not eligible to claim and create account");
        }
    }

    /// Create new account without linkdrop and deposit passed funds (used for creating sub accounts directly).
    #[payable]
    pub fn create_account(
        &mut self,
        new_account_id: ValidAccountId,
        new_public_key: Base58PublicKey,
    ) -> Promise {
        let amount = env::attached_deposit();
        Promise::new(new_account_id.to_string())
            .create_account()
            .add_full_access_key(new_public_key.into())
            .transfer(amount)
            .then(ext_self::on_account_created(
                env::predecessor_account_id(),
                amount.into(),
                &env::current_account_id(),
                NO_DEPOSIT,
                ON_CREATE_ACCOUNT_CALLBACK_GAS,
            ))
    }

    /// Callback after executing `create_account`.
    pub fn on_account_created(&mut self, predecessor_account_id: AccountId, amount: U128) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Callback can only be called from the contract"
        );
        let creation_succeeded = is_promise_success();
        if !creation_succeeded {
            // In case of failure, send attached deposit back.
            Promise::new(predecessor_account_id).transfer(amount.into());
        }
        creation_succeeded
    }

    /// Callback after execution `create_account_and_claim`.
    pub fn update_nft_storage(&mut self, public_key: PublicKey) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Callback can only be called from the contract"
        );
        let creation_succeeded = is_promise_success();
        if creation_succeeded {
            //removing key access to pk
            Promise::new(env::current_account_id()).delete_key(public_key);
        }
        creation_succeeded
    }

    // Method returns true is given pk is able to claim the reward
    pub fn public_key_is_claimable(&self, public_key: Base58PublicKey) -> bool {
        self.accounts.contains(&public_key.into())
    }
}

// TODO: Update tests

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use near_sdk::test_utils::accounts;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance, BlockHeight, PublicKey, VMContext};

    use super::*;

    pub struct VMContextBuilder {
        context: VMContext,
    }

    impl VMContextBuilder {
        pub fn new() -> Self {
            Self {
                context: VMContext {
                    current_account_id: "".to_string(),
                    signer_account_id: "".to_string(),
                    signer_account_pk: vec![0, 1, 2],
                    predecessor_account_id: "".to_string(),
                    input: vec![],
                    block_index: 0,
                    epoch_height: 0,
                    block_timestamp: 0,
                    account_balance: 0,
                    account_locked_balance: 0,
                    storage_usage: 10u64.pow(6),
                    attached_deposit: 0,
                    prepaid_gas: 10u64.pow(18),
                    random_seed: vec![0, 1, 2],
                    is_view: false,
                    output_data_receivers: vec![],
                },
            }
        }

        pub fn current_account_id(mut self, account_id: AccountId) -> Self {
            self.context.current_account_id = account_id;
            self
        }

        #[allow(dead_code)]
        pub fn signer_account_id(mut self, account_id: AccountId) -> Self {
            self.context.signer_account_id = account_id;
            self
        }

        pub fn predecessor_account_id(mut self, account_id: AccountId) -> Self {
            self.context.predecessor_account_id = account_id;
            self
        }

        #[allow(dead_code)]
        pub fn block_index(mut self, block_index: BlockHeight) -> Self {
            self.context.block_index = block_index;
            self
        }

        pub fn attached_deposit(mut self, amount: Balance) -> Self {
            self.context.attached_deposit = amount;
            self
        }

        pub fn account_balance(mut self, amount: Balance) -> Self {
            self.context.account_balance = amount;
            self
        }

        #[allow(dead_code)]
        pub fn account_locked_balance(mut self, amount: Balance) -> Self {
            self.context.account_locked_balance = amount;
            self
        }

        pub fn signer_account_pk(mut self, pk: PublicKey) -> Self {
            self.context.signer_account_pk = pk;
            self
        }

        pub fn finish(self) -> VMContext {
            self.context
        }
    }

    fn linkdrop() -> String {
        "linkdrop".to_string()
    }

    fn bob() -> ValidAccountId {
        accounts(1)
    }

    #[test]
    fn test_create_account() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        let deposit = 1_000_000;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .attached_deposit(deposit)
            .finish());
        contract.create_account(bob(), pk.clone());
        assert_eq!(contract.public_key_is_claimable(pk), false);
    }

    #[test]
    #[should_panic]
    fn test_create_invalid_account() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        let deposit = 1_000_000;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .attached_deposit(deposit)
            .finish());
        contract.create_account("XYZ".to_string().try_into().unwrap(), pk.clone());
        assert_eq!(contract.public_key_is_claimable(pk), false);
    }

    #[test]
    #[should_panic]
    fn test_create_account_and_claim_invalid_account() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        let token_id: TokenId = "0".try_into().unwrap();
        // Deposit money to linkdrop contract.
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .attached_deposit(deposit)
            .finish());
        contract.send(pk.clone());
        // Now, send new transaction to link drop contract.
        let context = VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .signer_account_pk(pk.clone().into())
            .account_balance(deposit)
            .finish();
        testing_env!(context);
        let pk2: Base58PublicKey = "2S87aQ1PM9o6eBcEXnTR5yBAVRTiNmvj8J8ngZ6FzSca"
            .try_into()
            .unwrap();
        contract.create_account_and_claim(
            "XYZ".to_string().try_into().unwrap(),
            pk2.clone(),
            token_id,
        );
        assert_eq!(contract.public_key_is_claimable(pk), true);
        assert_eq!(contract.public_key_is_claimable(pk2), false);
    }

    #[test]
    fn test_create_account_and_claim() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        let token_id: TokenId = "0".try_into().unwrap();
        // Deposit money to linkdrop contract.
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .attached_deposit(deposit)
            .finish());
        contract.send(pk.clone());
        assert_eq!(contract.public_key_is_claimable(pk.clone()), true);
        // Now, send new transaction to link drop contract.
        let context = VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .signer_account_pk(pk.into())
            .account_balance(deposit)
            .finish();
        testing_env!(context);
        let pk2 = "2S87aQ1PM9o6eBcEXnTR5yBAVRTiNmvj8J8ngZ6FzSca"
            .try_into()
            .unwrap();
        contract.create_account_and_claim(bob(), pk2, token_id);
        // TODO: verify that proper promises were created.
    }

    #[should_panic(expected = r#"Signer must be eligible to claim the NFT"#)]
    #[test]
    fn test_create_account_and_claim_pk_not_claimable() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        let token_id: TokenId = "0".try_into().unwrap();
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        // Now, send new transaction to link drop contract.
        let context = VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .signer_account_pk(pk.into())
            .account_balance(deposit)
            .finish();
        testing_env!(context);
        let pk2 = "2S87aQ1PM9o6eBcEXnTR5yBAVRTiNmvj8J8ngZ6FzSca"
            .try_into()
            .unwrap();
        contract.create_account_and_claim(bob(), pk2, token_id);
        // TODO: verify that proper promises were created.
    }

    #[test]
    #[should_panic(expected = r#"Account already registered"#)]
    fn test_send_two_times() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        // Deposit money to linkdrop contract.
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .finish());
        contract.send(pk.clone());
        assert_eq!(contract.public_key_is_claimable(pk.clone()), true);
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .account_balance(deposit)
            .finish());
        contract.send(pk.clone());
    }

    #[test]
    fn test_claim() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        // Deposit money to linkdrop contract.
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .attached_deposit(deposit)
            .finish());
        contract.send(pk.clone());
        assert_eq!(contract.public_key_is_claimable(pk.clone()), true);

        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .signer_account_pk(pk.clone().into())
            .attached_deposit(deposit)
            .finish());
        contract.claim(bob(), String::from("0"));
    }

    #[should_panic(expected = r#"Signer must be eligible to claim the NFT"#)]
    #[test]
    fn test_claim_pk_not_claimable() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        // Deposit money to linkdrop contract.
        let deposit = ACCESS_KEY_ALLOWANCE * 100;

        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .signer_account_pk(pk.clone().into())
            .attached_deposit(deposit)
            .finish());
        contract.claim(bob(), String::from("0"));
    }

    #[test]
    fn test_claim_invalid_current_account() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        let token_id: TokenId = "0".try_into().unwrap();
        // Deposit money to linkdrop contract.
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .attached_deposit(deposit)
            .finish());
        contract.send(pk.clone());
        assert_eq!(contract.public_key_is_claimable(pk.clone()), true);

        testing_env!(VMContextBuilder::new()
            .current_account_id(bob().try_into().unwrap())
            .predecessor_account_id(linkdrop())
            .signer_account_pk(pk.clone().into())
            .attached_deposit(deposit)
            .finish());
        contract.claim(bob(), token_id);
    }
    #[should_panic(expected = r#"The account ID is invalid"#)]
    #[test]
    fn test_claim_invalid_account() {
        let mut contract = LinkDrop::default();
        let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
            .try_into()
            .unwrap();
        let token_id: TokenId = "0".try_into().unwrap();
        // Deposit money to linkdrop contract.
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .attached_deposit(deposit)
            .finish());
        contract.send(pk.clone());
        assert_eq!(contract.public_key_is_claimable(pk.clone()), true);

        testing_env!(VMContextBuilder::new()
            .current_account_id(linkdrop())
            .predecessor_account_id(linkdrop())
            .signer_account_pk(pk.clone().into())
            .attached_deposit(deposit)
            .finish());
        contract.claim("XYZ".to_string().try_into().unwrap(), token_id);
    }
}
