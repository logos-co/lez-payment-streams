use crate::test_helpers::create_keypair;

#[test]
fn vault_tests_trivial_test() {
    assert!(true);
}

#[test]
fn vault_tests_keypair_is_deterministic_for_seed() {
    let (_, first) = create_keypair(7);
    let (_, second) = create_keypair(7);
    assert_eq!(first, second);
}
