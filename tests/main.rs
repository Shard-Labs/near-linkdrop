use linkdrop::LinkDropContract;
use near_contract_standards::non_fungible_token::metadata::{NFTContractMetadata, TokenMetadata};
use near_sdk::AccountId;
use near_sdk_sim::near_crypto::{InMemorySigner, KeyType};
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, ContractAccount, UserAccount};
use std::convert::TryInto;

const EVENT_SEND_ALLOWANCE: u128 = 7_000_000_000_000_000_000_000_000;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    LINKDROP_WASM_BYTES => "res/linkdrop.wasm"
}

fn init() -> (UserAccount, ContractAccount<LinkDropContract>) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = 300_000_000_000_000;
    genesis.gas_price = 1;
    let master_account = init_simulator(Some(genesis));

    let linkdrop = deploy! {
        contract: LinkDropContract,
        contract_id: get_linkdrop(),
        bytes: &LINKDROP_WASM_BYTES,
        signer_account: master_account
    };
    call!(master_account, linkdrop.init(get_nft())).assert_success();
    (master_account, linkdrop)
}

#[test]
fn simulate_claim() {
    let (master_account, linkdrop) = init();

    let alice = master_account.create_user(get_alice(), to_yocto("100"));
    call!(
        alice,
        linkdrop.send(
            vec![linkdrop
                .user_account
                .signer
                .public_key
                .to_string()
                .try_into()
                .unwrap()],
            vec![String::from("0")]
        ),
        deposit = EVENT_SEND_ALLOWANCE
    )
    .assert_success();

    let res = call!(
        linkdrop.user_account,
        linkdrop.claim(get_bob().try_into().unwrap())
    );
    res.assert_success();

    println!("{:#?}\n", res);
}

#[test]
fn simulate_create_account_and_claim() {
    let (master_account, linkdrop) = init();

    let alice = master_account.create_user(get_alice(), to_yocto("100"));
    call!(
        alice,
        linkdrop.send(
            vec![linkdrop
                .user_account
                .signer
                .public_key
                .to_string()
                .try_into()
                .unwrap()],
            vec![String::from("0")]
        ),
        deposit = EVENT_SEND_ALLOWANCE
    )
    .assert_success();

    let receiver_id = InMemorySigner::from_seed("bob", KeyType::ED25519, "receiver_id");
    let res = call!(
        linkdrop.user_account,
        linkdrop.create_account_and_claim(
            get_bob().try_into().unwrap(),
            receiver_id.public_key.to_string().try_into().unwrap()
        )
    );
    res.assert_success();
    println!("{:#?}\n{:#?}\n", res, res.promise_results());
}

fn get_nft() -> AccountId {
    String::from("nft")
}

fn get_linkdrop() -> AccountId {
    String::from("linkdrop")
}

fn get_alice() -> AccountId {
    String::from("alice")
}

fn get_bob() -> AccountId {
    String::from("bob")
}
