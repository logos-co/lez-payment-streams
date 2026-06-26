{
  description = "Step 18 LEZ submit helper (CHAIN=testnet; same rc5 pin as scaffold)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    lez.url = "github:logos-blockchain/logos-execution-zone/27360cb7d6ccb2bfbcca7d171bab8a3938490264";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      lez,
      rust-overlay,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems =
        f: nixpkgs.lib.genAttrs systems (system: f { pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; }; });
    in
    {
      packages = forAllSystems (
        { pkgs }:
        let
          rustPlatform = pkgs.makeRustPlatform {
            cargoBin = pkgs.rust-bin.stable.latest.minimal;
            rustc = pkgs.rust-bin.stable.latest.minimal;
          };
          lezSrc = lez;
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "lez-testnet-submit";
            version = "0.1.0";
            src = pkgs.lib.cleanSource ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = with pkgs; [
              pkg-config
              openssl
            ];
            buildInputs = with pkgs; [
              openssl
            ];
            postPatch = ''
              mkdir -p vendor/lez
              cp -r ${lezSrc}/* vendor/lez/
            '';
            preBuild = ''
              cat > .cargo/config.toml <<EOF
[patch."https://github.com/logos-blockchain/logos-execution-zone"]
wallet = { path = "vendor/lez/wallet" }
nssa = { path = "vendor/lez/nssa" }
common = { path = "vendor/lez/common" }
EOF
            '';
            doCheck = true;
          };
        }
      );
    };
}
