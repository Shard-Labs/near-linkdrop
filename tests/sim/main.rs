use linkdrop::LinkDropContract;
use near_sdk::json_types::Base58PublicKey;
use near_sdk_sim::{call, deploy, init_simulator, view, ContractAccount, UserAccount};
use std::convert::TryInto;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    LINKDROP_BYTES => "target/wasm32-unknown-unknown/release/linkdrop.wasm"
}

pub const DEFAULT_GAS: u128 = 1_000_000_000_000_000_000_000_001;

fn init() -> (UserAccount, ContractAccount<LinkDropContract>) {
    let root = init_simulator(None);
    // Deploy the compiled Wasm bytes
    let linkdrop = deploy!(
        contract: LinkDropContract,
        contract_id: "linkdrop",
        bytes: &LINKDROP_BYTES,
        signer_account: root
    );
    (root, linkdrop)
}

#[test]
fn simulate_send() {
    let (root, linkdrop) = init();
    let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
        .try_into()
        .unwrap();
    call!(root, linkdrop.send(pk), deposit = DEFAULT_GAS).assert_success();
}

#[should_panic(expected = r#"Account already registered"#)]
#[test]
fn simulate_send_duplicated() {
    let (root, linkdrop) = init();
    let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
        .try_into()
        .unwrap();
    call!(root, linkdrop.send(pk.clone()), deposit = DEFAULT_GAS).assert_success();
    call!(root, linkdrop.send(pk), deposit = DEFAULT_GAS).assert_success();
}

#[should_panic(expected = r#"Attached deposit must be greater than ACCESS_KEY_ALLOWANCE"#)]
#[test]
fn simulate_send_insuficient_attached_deposit() {
    let (root, linkdrop) = init();
    let pk: Base58PublicKey = "qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz"
        .try_into()
        .unwrap();
    call!(root, linkdrop.send(pk.clone()), deposit = 1).assert_success();
}
