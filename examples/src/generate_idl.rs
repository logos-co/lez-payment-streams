// Generate IDL JSON for the lez-payment-streams program.
//
// Usage (from repo root):
//   make idl
//
// Path is relative to this crate's manifest dir (examples/); spel also emits
// include_str! paths that must resolve from this file's directory (examples/src/).

spel_framework::generate_idl!("../methods/guest/src/bin/lez_payment_streams.rs");
