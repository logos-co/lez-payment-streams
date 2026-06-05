{
  description = "payment_streams_module Logos Core plugin";

  inputs = {
    logos-module-builder.url = "github:logos-co/logos-module-builder";
    lez-payment-streams.url = "path:..";

    # LEZ PR 491 — https://github.com/logos-blockchain/logos-execution-zone/pull/491
    logos-execution-zone.url = "github:logos-blockchain/lssa?ref=refs/pull/491/head";

    # Wallet module PR 19 via patched wrapper — https://github.com/logos-blockchain/logos-execution-zone-module/pull/19
    logos-execution-zone-module.url = "path:./nix/flakes/logos-execution-zone-module-patched";
    logos-execution-zone-module.inputs.logos-execution-zone.follows = "logos-execution-zone";
    # Patched flake runs logos-cpp-generator on the wallet plugin; pin SDK and nixpkgs with the builder.
    logos-execution-zone-module.inputs.logos-cpp-sdk.follows = "logos-module-builder/logos-cpp-sdk";
    logos-execution-zone-module.inputs.nixpkgs.follows = "logos-module-builder/nixpkgs";
  };

  outputs =
    inputs@{ logos-module-builder, logos-execution-zone-module, lez-payment-streams, ... }:
    logos-module-builder.lib.mkLogosModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs // {
        lez_wallet_module = logos-execution-zone-module;
      };
      externalLibInputs = {
        lez_payment_streams_ffi = {
          input = lez-payment-streams;
          packages.default = "payment-streams-ffi";
        };
      };
    };
}
