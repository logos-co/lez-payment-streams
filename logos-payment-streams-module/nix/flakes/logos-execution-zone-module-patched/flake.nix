{
  description = "logos-execution-zone-module PR 16 + CMake include fix for wallet_ffi.h";

  inputs = {
    logos-execution-zone.url = "github:logos-blockchain/lssa?ref=refs/pull/429/head";

    upstream.url = "github:logos-blockchain/logos-execution-zone-module?ref=refs/pull/16/head";
    upstream.inputs.logos-execution-zone.follows = "logos-execution-zone";

    nixpkgs.follows = "upstream/logos-liblogos/nixpkgs";
    logos-cpp-sdk.url = "github:logos-co/logos-cpp-sdk";
  };

  outputs =
    inputs@{ upstream, nixpkgs, logos-cpp-sdk, ... }:
    let
      patchWalletInclude = drv: drv.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ ./cmake-wallet-ffi-include.patch ];
        postPatch =
          (old.postPatch or "")
          + ''
            # Keep stable Logos module id + codegen naming expected by logos-module-builder.
            substituteInPlace metadata.json \
              --replace '"liblogos_execution_zone_wallet_module"' '"lez_wallet_module"'
          '';
        postInstall =
          (old.postInstall or "")
          + ''
            # logos-module-builder / logos-cpp-generator resolves dependency modules using metadata.json.
            install -Dm644 "$src/metadata.json" "$out/metadata.json"

            # logos-module-builder uses "<module>_plugin.<shlibExt>" without a leading lib/ prefix.
            ln -sfn liblogos_execution_zone_wallet_module.so "$out/lib/lez_wallet_module_plugin.so"
          '';
      });

      # PR 16 builds the Qt plugin with plain CMake (no mkLogosModule). Downstream modules expect
      # include/lez_wallet_module_api.{h,cpp} (see logos-plugin-qt lib/buildPlugin.nix dependency copy).
      addSdkApiHeaders = system: base:
        let
          pkgs = import nixpkgs { inherit system; };
          logosSdk = logos-cpp-sdk.packages.${system}.default;
        in
        pkgs.runCommand "${base.name}-with-sdk-api-headers" {
          nativeBuildInputs = [ logosSdk ];
        } ''
          cp -a "${base}/." "$out/"
          chmod -R u+w "$out"
          mkdir gen
          export LD_LIBRARY_PATH="$out/lib"
          logos-cpp-generator "$out/lib/lez_wallet_module_plugin.so" --output-dir gen --module-only
          install -Dm644 gen/lez_wallet_module_api.h "$out/include/lez_wallet_module_api.h"
          install -Dm644 gen/lez_wallet_module_api.cpp "$out/include/lez_wallet_module_api.cpp"
        '';

      mapSystemPackages =
        system: pkgsForSys:
        let
          baseLib = patchWalletInclude pkgsForSys.lib;
          wrapped = addSdkApiHeaders system baseLib;
        in
        builtins.mapAttrs (
          name: drv:
          if
            (builtins.typeOf drv == "set")
            && (builtins.hasAttr "overrideAttrs" drv)
            && ((name == "default") || (name == "lib"))
          then wrapped
          else drv
        ) pkgsForSys;
    in
    {
      packages = builtins.mapAttrs mapSystemPackages upstream.packages;

      apps = upstream.apps or { };

      devShells = upstream.devShells or { };
    };
}
