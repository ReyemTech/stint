use stint_core::oauth::pkce::{code_challenge_for, generate_verifier};

#[test]
fn verifier_is_43_to_128_chars_of_allowed_alphabet() {
    let v = generate_verifier();
    let len = v.len();
    assert!(
        (43..=128).contains(&len),
        "verifier length {len} out of range"
    );
    assert!(
        v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')),
        "verifier contains disallowed char: {v}"
    );
}

#[test]
fn two_verifiers_in_a_row_are_distinct() {
    let a = generate_verifier();
    let b = generate_verifier();
    assert_ne!(
        a, b,
        "PRNG produced two identical verifiers — high entropy lost?"
    );
}

#[test]
fn challenge_is_base64url_sha256_of_verifier() {
    // Known test vector from RFC 7636 §4.4.
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    assert_eq!(code_challenge_for(verifier), expected_challenge);
}
