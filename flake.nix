{
  description = "lez-payment-streams workspace (FFI packaging for Logos modules)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      eachSystem = nixpkgs.lib.genAttrs systems;
      paymentStreamsFfi =
        {
          lib,
          rustPlatform,
          fetchFromGitHub,
          pkg-config,
          openssl,
          cacert,
          gitMinimal,
        }:
        let
          # LEZ v0.2.0 — program-graph pin. Operator scaffold is v0.2.4. See docs/reference/pins.md.
          logosExecutionZoneSrc = fetchFromGitHub {
            owner = "logos-blockchain";
            repo = "logos-execution-zone";
            rev = "a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a";
            sha256 = "sha256-OnXBx3nD/r7vCzZwh/RXmoqbtNF9rG+ZZsWXPsXoOzk=";
          };
        in
        rustPlatform.buildRustPackage rec {
          pname = "lez-payment-streams-ffi";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          cargoBuildFlags = [
            "--package"
            "lez-payment-streams-ffi"
          ];

          nativeBuildInputs = [
            pkg-config
            gitMinimal
            cacert
          ];
          buildInputs = [ openssl ];

          doCheck = false;

          preBuild = ''
            vendor_root="''${NIX_BUILD_TOP:-/build}/cargo-vendor-dir"
            ln -sfn ${logosExecutionZoneSrc}/artifacts "$vendor_root/artifacts"
            ln -sfn ${logosExecutionZoneSrc}/artifacts "''${NIX_BUILD_TOP:-/build}/artifacts"
          '';

          postInstall = ''
            mkdir -p $out/include
            cp "$src/module/ffi/lez_payment_streams_ffi.h" "$out/include/"
          '';

          meta = with lib; {
            description = "C FFI shared library for lez-payment-streams";
            license = licenses.mit;
          };
        };
    in
    {
      packages = eachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          payment-streams-ffi = pkgs.callPackage paymentStreamsFfi { };
        }
      );
    };
}
