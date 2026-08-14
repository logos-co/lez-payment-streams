// Legacy entrypoint retained for discovery.
// Step 45: `make idl` uses stock `spel generate-idl` (tag v0.6.0 path) so we
// do not depend on `generate_idl!` include_str path quirks from examples/src.

fn main() {
    eprintln!(
        "use: make idl  (spel generate-idl program/methods/guest/src/bin/lez_payment_streams.rs)"
    );
    std::process::exit(2);
}
