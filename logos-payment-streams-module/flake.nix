{
  description = "payment_streams_module Logos Core plugin";

  inputs = {
    logos-module-builder.url = "github:logos-co/logos-module-builder";
    lez-payment-streams.url = "path:..";

    # https://github.com/logos-blockchain/logos-execution-zone/pull/429
    # Module flake names this input logos-execution-zone but points it at logos-blockchain/lssa.
    logos-execution-zone.url = "github:logos-blockchain/lssa?ref=refs/pull/429/head";

    # Wallet module from PR 16, built against LEZ PR 429 (see patched wrapper flake).
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
