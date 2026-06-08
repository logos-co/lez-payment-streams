{
  description = "logos-execution-zone: python3 for pyo3 + correct wallet_ffi.h install path";

  inputs.lez.url = "github:logos-blockchain/lssa?rev=c37a3c30a96515cba756174da1da4137ff025d7f";
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
