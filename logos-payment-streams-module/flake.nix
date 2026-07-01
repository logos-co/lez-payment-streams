{
  description = "payment_streams_module Logos Core module (Universal)";

  inputs = {
    logos-module-builder.url = "github:logos-co/logos-module-builder";
    lez-payment-streams.url = "path:..";
    logos_execution_zone.url = "path:./nix/flakes/logos-execution-zone-module-patched";
  };

  outputs =
    inputs@{ logos-module-builder, lez-payment-streams, ... }:
    logos-module-builder.lib.mkLogosModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
      externalLibInputs = {
        lez_payment_streams_ffi = {
          input = lez-payment-streams;
          packages.default = "payment-streams-ffi";
        };
      };
    };
}
