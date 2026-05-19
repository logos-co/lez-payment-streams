{
  description = "payment_streams_module Logos Core plugin";

  inputs = {
    logos-module-builder.url = "github:logos-co/logos-module-builder";
    logos-execution-zone-module.url = "github:logos-blockchain/logos-execution-zone-module";
    lez-payment-streams.url = "path:..";
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
