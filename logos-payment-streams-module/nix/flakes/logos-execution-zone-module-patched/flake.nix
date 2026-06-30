{
  description = "logos-execution-zone-module upstream main (Universal) + payment-streams wallet patches (LEZ v0.2.0)";

  inputs = {
    # pyo3-build-config in wallet-ffi-deps needs a Python interpreter in nativeBuildInputs.
    logos-execution-zone.url = "path:./lez-wallet-ffi-patched";

    upstream.url = "github:logos-blockchain/logos-execution-zone-module";
    upstream.inputs.logos-execution-zone.follows = "logos-execution-zone";

    nixpkgs.follows = "upstream/logos-module-builder/nixpkgs";
    logos-cpp-sdk.url = "github:logos-co/logos-cpp-sdk";
  };

  outputs =
    inputs@{ upstream, nixpkgs, logos-cpp-sdk, ... }:
    let
      # Bundler (nix-bundle-lgx) reads metadata from drv.src at eval time; upstream wallet build is not mkLogosModule.
      walletMetadataJson = builtins.toJSON {
        name = "logos_execution_zone";
        version = "1.0.0";
        description = "Logos Execution Zone Module for Logos Core";
        author = "Logos Blockchain Team";
        type = "core";
        category = "blockchain";
        main = "logos_execution_zone_plugin";
        dependencies = [ ];
        capabilities = [ ];
      };

      patchWalletInclude = drv: drv.overrideAttrs (old: {
        postPatch =
          (old.postPatch or "")
          + ''
            patch -p1 --forward < ${./wallet-qt-sign-public-payload.patch}
            patch -p1 --forward < ${./wallet-qt-send-generic-public-transaction-json.patch}
          '';
      });

      # Upstream PR 19 builds the Qt plugin with plain CMake (no mkLogosModule). Downstream modules expect
      # include/logos_execution_zone_api.{h,cpp} (see logos-plugin-qt lib/buildPlugin.nix dependency copy).
      addSdkApiHeaders = system: base:
        let
          pkgs = import nixpkgs { inherit system; };
          logosSdk = logos-cpp-sdk.packages.${system}.default;
          # nix-bundle-lgx reads metadata.json from drv.src at eval time, not from $out.
          bundleSrc = pkgs.runCommand "logos-execution-zone-wallet-bundle-src" { } ''
            mkdir -p $out
            cp ${pkgs.writeText "logos-execution-zone-wallet-metadata.json" walletMetadataJson} $out/metadata.json
          '';
        in
        (pkgs.runCommand "${base.name}-with-sdk-api-headers" {
          nativeBuildInputs = [ logosSdk ];
        } ''
          cp -a "${base}/." "$out/"
          chmod -R u+w "$out"
          mkdir gen
          export LD_LIBRARY_PATH="$out/lib"
          logos-cpp-generator "$out/lib/logos_execution_zone_plugin.so" --output-dir gen --module-only
          install -Dm644 gen/logos_execution_zone_api.h "$out/include/logos_execution_zone_api.h"
          install -Dm644 gen/logos_execution_zone_api.cpp "$out/include/logos_execution_zone_api.cpp"
        '') // {
          src = bundleSrc;
        };

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
