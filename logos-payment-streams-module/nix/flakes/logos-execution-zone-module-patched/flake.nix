{
  description = "logos-execution-zone-module upstream main (Universal) + payment-streams wallet patches (LEZ v0.2.0)";

  inputs = {
    # pyo3-build-config in wallet-ffi-deps needs a Python interpreter in nativeBuildInputs.
    logos-execution-zone.url = "path:./lez-wallet-ffi-patched";

    upstream.url = "github:logos-blockchain/logos-execution-zone-module/92dd9e25bcc6be04f841671e8da7b94bd2449f39";
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
            patch -p1 --forward < ${./wallet-qt-sign-private-payload.patch}
            patch -p1 --forward < ${./wallet-qt-send-generic-public-transaction-json.patch}
            patch -p1 --forward < ${./wallet-qt-send-generic-private-transaction-json.patch}
            patch -p1 --forward < ${./wallet-qt-private-json-elf-path.patch}
            patch -p1 --forward < ${./wallet-qt-transfer-shielded-amount-prefix.patch}
            patch -p1 --forward < ${./wallet-qt-fix-authenticated-transfer-elf.patch}
            patch -p1 --forward < ${./wallet-qt-cmake-module-name.patch}
            patch -p1 --forward < ${./wallet-qt-metadata-module-name.patch}
          '';
        # The module-builder templates installPhase at eval time from the
        # upstream metadata name (lez_core), so it looks for
        # modules/lez_core_plugin.so. Our CMake NAME override builds
        # logos_execution_zone_plugin.so instead. Bridge the two without
        # forking the whole installPhase: let installPhase find a symlink,
        # then rename the installed file to the identity we preserve.
        preInstall = ''
          if [ ! -e modules/lez_core_plugin.so ] && [ -e modules/logos_execution_zone_plugin.so ]; then
            ln -s logos_execution_zone_plugin.so modules/lez_core_plugin.so
          fi
        '';
        postInstall = ''
          if [ -e "$out/lib/lez_core_plugin.so" ] && [ ! -e "$out/lib/logos_execution_zone_plugin.so" ]; then
            mv "$out/lib/lez_core_plugin.so" "$out/lib/logos_execution_zone_plugin.so"
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
          # The lidl sidecar derivation is templated from the upstream eval-time
          # name (lez_core.lidl). Downstream depends on logos_execution_zone, so
          # republish the sidecar under the identity we preserve.
          renameLidl = base:
            let
              pkgs = import nixpkgs { inherit system; };
            in
            pkgs.runCommand "${base.name}-as-logos_execution_zone" { } ''
              mkdir -p $out
              cp "${base}/lez_core.lidl" "$out/logos_execution_zone.lidl"
            '';
        in
        builtins.mapAttrs (
          name: drv:
          if
            (builtins.typeOf drv == "set")
            && (builtins.hasAttr "overrideAttrs" drv)
            && ((name == "default") || (name == "lib"))
          then wrapped
          else if
            (builtins.typeOf drv == "set")
            && (builtins.hasAttr "overrideAttrs" drv)
            && ((name == "lidl") || (name == "lez_core-lidl"))
          then renameLidl drv
          else drv
        ) pkgsForSys;
    in
    {
      packages = builtins.mapAttrs mapSystemPackages upstream.packages;

      apps = upstream.apps or { };

      devShells = upstream.devShells or { };
    };
}
