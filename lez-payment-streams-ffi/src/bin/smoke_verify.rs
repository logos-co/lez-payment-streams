// Smoke-test helper: verify a Schnorr signature over a 32-byte digest.
//
// Usage:
//   smoke_verify <pubkey_hex_32> <digest_hex_32> <sig_hex_64>
//
// Exits 0 on valid, 1 on invalid signature, 2 on bad input.
// Used by verify-step11c-dod.sh to round-trip wallet sign_public_payload output.
// Uses the same verify_canonical_payload_digest path as the provider (Step 13).

use lez_payment_streams_core::verify_canonical_payload_digest;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        let prog = args.get(0).map(String::as_str).unwrap_or("smoke_verify");
        eprintln!("usage: {prog} <pubkey_hex_32> <digest_hex_32> <sig_hex_64>");
        std::process::exit(2);
    }

    let pubkey = decode_fixed::<32>(args.get(1).expect("checked len == 4"), "pubkey");
    let digest = decode_fixed::<32>(args.get(2).expect("checked len == 4"), "digest");
    let sig = decode_fixed::<64>(args.get(3).expect("checked len == 4"), "signature");

    match verify_canonical_payload_digest(&digest, &sig, &pubkey) {
        Ok(()) => {
            println!("ok");
        }
        Err(e) => {
            eprintln!("invalid: {e:?}");
            std::process::exit(1);
        }
    }
}

fn decode_fixed<const N: usize>(hex_str: &str, label: &str) -> [u8; N] {
    let bytes = match hex::decode(hex_str) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{label}: invalid hex: {e}");
            std::process::exit(2);
        }
    };
    if bytes.len() != N {
        eprintln!("{label}: expected {N} bytes, got {}", bytes.len());
        std::process::exit(2);
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&bytes);
    arr
}
