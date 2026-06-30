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
  # LEZ v0.2.0 — operational pin (matches scaffold.toml).
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

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  cargoBuildFlags = [ "--package" "lez-payment-streams-ffi" ];

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
    cp "$src/lez-payment-streams-ffi/lez_payment_streams_ffi.h" "$out/include/"
  '';

  meta = with lib; {
    description = "C FFI shared library for lez-payment-streams";
    license = licenses.mit;
  };
}
