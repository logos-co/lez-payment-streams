{
  description = "payment_streams_ui Logos QML plugin";

  inputs = {
    logos-module-builder.url = "github:logos-co/logos-module-builder";
  };

  outputs =
    inputs@{ logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosQmlModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
