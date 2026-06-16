{
  description = "logos-execution-zone: python3 for pyo3 + correct wallet_ffi.h install path";

  inputs.lez.url = "github:logos-blockchain/logos-execution-zone?rev=a8c81f5445166b22672a614b159a1c38a5907a65";
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
              lbcPolLib =
                if builtins.pathExists /tmp/lbc-pol-v0.5.0/logos-blockchain-circuits-v0.5.0-linux-x86_64/pol then
                  builtins.path {
                    path = /tmp/lbc-pol-v0.5.0/logos-blockchain-circuits-v0.5.0-linux-x86_64/pol;
                    name = "lbc-pol-lib";
                  }
                else
                  null;
              lbcPolExport =
                if lbcPolLib == null then "" else ''
                  export LBC_POL_LIB_DIR=${lbcPolLib}
                '';
              # Upstream flake copies wallet-ffi/wallet_ffi.h; crate lives under lez/wallet-ffi/.
              walletFfiHeaderPostInstall = ''
                mkdir -p $out/include
                cp lez/wallet-ffi/wallet_ffi.h $out/include/
              '';
              addPythonToWalletDeps =
                drv:
                drv.overrideAttrs (old: {
                  nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ pkgs.python3 ];
                  postInstall = walletFfiHeaderPostInstall;
                  cargoArtifacts = old.cargoArtifacts.overrideAttrs (deps: {
                    nativeBuildInputs = (deps.nativeBuildInputs or [ ]) ++ [ pkgs.python3 ];
                    postInstall = walletFfiHeaderPostInstall;
                    preBuild = (deps.preBuild or "") + lbcPolExport;
                    env = (deps.env or { }) // {
                      LBC_POL_LIB_DIR =
                        if lbcPolLib == null then null else "${lbcPolLib}";
                    };
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
