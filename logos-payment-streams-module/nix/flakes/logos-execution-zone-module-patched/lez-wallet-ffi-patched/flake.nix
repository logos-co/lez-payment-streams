{
  description = "logos-execution-zone wallet-ffi patched: python3 for pyo3 + wallet_ffi.h install path + payment-streams Rust patches";

  # LEZ v0.2.0 — operational pin (local E2E + public testnet wallet).
  inputs.lez.url = "github:logos-blockchain/logos-execution-zone?rev=a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a";
  inputs.nixpkgs.follows = "lez/nixpkgs";

  outputs =
    { lez, nixpkgs, ... }:
    {
      packages =
        builtins.mapAttrs
          (
            system:
            lezSys:
            let
              pkgs = import nixpkgs { inherit system; };
              lbcBase = /tmp/lbc-pol-v0.5.0/logos-blockchain-circuits-v0.5.0-linux-x86_64;
              # lbcBaseLib points at the lib/ subdir which provides libgmp.a and other shared libs.
              lbcBaseLib =
                if builtins.pathExists (lbcBase + /lib) then
                  builtins.path { path = lbcBase + /lib; name = "lbc-libs"; }
                else
                  null;
              lbcPolLib =
                if builtins.pathExists (lbcBase + /pol) then
                  builtins.path { path = lbcBase + /pol; name = "lbc-pol-lib"; }
                else
                  null;
              lbcPoqLib =
                if builtins.pathExists (lbcBase + /poq) then
                  builtins.path { path = lbcBase + /poq; name = "lbc-poq-lib"; }
                else
                  null;
              lbcPocLib =
                if builtins.pathExists (lbcBase + /poc) then
                  builtins.path { path = lbcBase + /poc; name = "lbc-poc-lib"; }
                else
                  null;
              lbcSigLib =
                if builtins.pathExists (lbcBase + /signature) then
                  builtins.path { path = lbcBase + /signature; name = "lbc-sig-lib"; }
                else
                  null;
              lbcPolExport =
                (if lbcBaseLib == null then "" else "export LBC_LIB_DIR=${lbcBaseLib}\n")
                + (if lbcPolLib == null then "" else "export LBC_POL_LIB_DIR=${lbcPolLib}\n")
                + (if lbcPoqLib == null then "" else "export LBC_POQ_LIB_DIR=${lbcPoqLib}\n")
                + (if lbcPocLib == null then "" else "export LBC_POC_LIB_DIR=${lbcPocLib}\n")
                + (if lbcSigLib == null then "" else "export LBC_SIGNATURE_LIB_DIR=${lbcSigLib}\n");
              # Upstream flake copies wallet-ffi/wallet_ffi.h; crate lives under lez/wallet-ffi/.
              walletFfiHeaderPostInstall = ''
                mkdir -p $out/include
                cp lez/wallet-ffi/wallet_ffi.h $out/include/
              '';
              lbcEnvAttrs =
                (if lbcBaseLib == null then { } else { LBC_LIB_DIR = "${lbcBaseLib}"; })
                // (if lbcPolLib == null then { } else { LBC_POL_LIB_DIR = "${lbcPolLib}"; })
                // (if lbcPoqLib == null then { } else { LBC_POQ_LIB_DIR = "${lbcPoqLib}"; })
                // (if lbcPocLib == null then { } else { LBC_POC_LIB_DIR = "${lbcPocLib}"; })
                // (if lbcSigLib == null then { } else { LBC_SIGNATURE_LIB_DIR = "${lbcSigLib}"; });
              addPythonToWalletDeps =
                drv:
                drv.overrideAttrs (old: {
                  nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ pkgs.python3 ];
                  # Apply source patches here (main build uses full source, not the stub from cargoArtifacts).
                  postPatch =
                    (old.postPatch or "")
                    + ''
                      patch -p1 --forward < ${./lez-rust-sign-public-payload.patch}
                    '';
                  postInstall = walletFfiHeaderPostInstall;
                  preBuild = (old.preBuild or "") + lbcPolExport;
                  env = (old.env or { }) // lbcEnvAttrs;
                  cargoArtifacts = old.cargoArtifacts.overrideAttrs (deps: {
                    nativeBuildInputs = (deps.nativeBuildInputs or [ ]) ++ [ pkgs.python3 ];
                    # cargoArtifacts uses cleanCargoSource (stub lib.rs only); no source patch here.
                    postInstall = walletFfiHeaderPostInstall;
                    preBuild = (deps.preBuild or "") + lbcPolExport;
                    env = (deps.env or { }) // lbcEnvAttrs;
                  });
                });
            in
            lezSys
            // builtins.listToAttrs (
              map
                (name: {
                  name = name;
                  value = addPythonToWalletDeps lezSys.${name};
                })
                (builtins.filter (n: lezSys.${n} ? cargoArtifacts) (builtins.attrNames lezSys))
            )
          )
          lez.packages;
    };
}
