// Generate IDL JSON for the lez-payment-streams program.
//
// Usage:
//   cargo run --bin generate_idl > lez-payment-streams-idl.json

spel_framework::generate_idl!("../methods/guest/src/bin/lez_payment_streams.rs");
