{
  description = "logos-execution-zone-module PR 19 + payment-streams wallet patches (LEZ main / PR 510)";

  inputs = {
    # pyo3-build-config in wallet-ffi-deps needs a Python interpreter in nativeBuildInputs.
    logos-execution-zone.url = "path:./lez-wallet-ffi-patched";

    upstream.url = "github:logos-blockchain/logos-execution-zone-module?ref=refs/pull/19/head";
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
            patch -p1 --forward < ${./wallet-qt-guest-elf-from-env.patch}
            patch -p1 --forward < ${./wallet-qt-sign-public-payload.patch}
            patch -p1 --forward < ${./wallet-qt-send-generic-public-transaction-json.patch}
          '';
        postInstall =
          (old.postInstall or "")
          + ''
            cat > "$out/metadata.json" <<'WALLET_METADATA_EOF'
${walletMetadataJson}
WALLET_METADATA_EOF

            _plugin=""
            for _c in logos_execution_zone_plugin.so liblogos_execution_zone_plugin.so \
              liblogos_execution_zone_wallet_module.so; do
              if [ -f "$out/lib/$_c" ]; then _plugin="$_c"; break; fi
            done
            if [ -z "$_plugin" ]; then
              echo "No wallet plugin .so under $out/lib:" >&2
              ls -la "$out/lib" >&2 || true
              exit 1
            fi
            if [ "$_plugin" != "logos_execution_zone_plugin.so" ]; then
              ln -sfn "$_plugin" "$out/lib/logos_execution_zone_plugin.so"
            fi
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
