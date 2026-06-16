{
  description = "logos-execution-zone-module PR 19 + lez_wallet_module packaging (LEZ main / 491 merged)";

  inputs = {
    # pyo3-build-config in wallet-ffi-deps needs a Python interpreter in nativeBuildInputs.
    logos-execution-zone.url = "path:./lez-python-overlay";

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
        name = "lez_wallet_module";
        version = "1.0.0";
        description = "Logos Execution Zone Wallet Module for Logos Core";
        author = "Logos Blockchain Team";
        type = "core";
        category = "blockchain";
        main = "lez_wallet_module_plugin";
        dependencies = [ ];
        capabilities = [ ];
      };

      patchWalletInclude = drv: drv.overrideAttrs (old: {
        postPatch =
          (old.postPatch or "")
          + ''
            # Stable Logos module id expected by logoscore / lgpm / downstream codegen.
            substituteInPlace metadata.json \
              --replace '"name": "logos_execution_zone"' '"name": "lez_wallet_module"' \
              --replace '"main": "logos_execution_zone_plugin"' '"main": "lez_wallet_module_plugin"'
            # logos_host registers the Qt Remote Object under PluginInterface::name().
            sed -i '/LogosExecutionZoneWalletModule::name() const/,/^}/ s/return "[^"]*";/return "lez_wallet_module";/' \
              src/logos_execution_zone_wallet_module.cpp
            patch -p1 --forward < ${./wallet-guest-elf-from-env.patch}
          '';
        postInstall =
          (old.postInstall or "")
          + ''
            cat > "$out/metadata.json" <<'WALLET_METADATA_EOF'
${walletMetadataJson}
WALLET_METADATA_EOF

            _plugin=""
            for _c in logos_execution_zone_plugin.so liblogos_execution_zone_plugin.so \
              liblogos_execution_zone_wallet_module.so lez_wallet_module_plugin.so; do
              if [ -f "$out/lib/$_c" ]; then _plugin="$_c"; break; fi
            done
            if [ -z "$_plugin" ]; then
              echo "No wallet plugin .so under $out/lib:" >&2
              ls -la "$out/lib" >&2 || true
              exit 1
            fi
            ln -sfn "$_plugin" "$out/lib/lez_wallet_module_plugin.so"
          '';
      });

      # Upstream PR 19 builds the Qt plugin with plain CMake (no mkLogosModule). Downstream modules expect
      # include/lez_wallet_module_api.{h,cpp} (see logos-plugin-qt lib/buildPlugin.nix dependency copy).
      addSdkApiHeaders = system: base:
        let
          pkgs = import nixpkgs { inherit system; };
          logosSdk = logos-cpp-sdk.packages.${system}.default;
          # nix-bundle-lgx reads metadata.json from drv.src at eval time, not from $out.
          bundleSrc = pkgs.runCommand "lez-wallet-module-bundle-src" { } ''
            mkdir -p $out
            cp ${pkgs.writeText "lez-wallet-module-metadata.json" walletMetadataJson} $out/metadata.json
          '';
        in
        (pkgs.runCommand "${base.name}-with-sdk-api-headers" {
          nativeBuildInputs = [ logosSdk ];
        } ''
          cp -a "${base}/." "$out/"
          chmod -R u+w "$out"
          mkdir gen
          export LD_LIBRARY_PATH="$out/lib"
          logos-cpp-generator "$out/lib/lez_wallet_module_plugin.so" --output-dir gen --module-only
          install -Dm644 gen/lez_wallet_module_api.h "$out/include/lez_wallet_module_api.h"
          install -Dm644 gen/lez_wallet_module_api.cpp "$out/include/lez_wallet_module_api.cpp"
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
