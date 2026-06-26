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
  # LEZ v0.2.0-rc5 — operational pin (matches scaffold.toml).
  logosExecutionZoneSrc = fetchFromGitHub {
    owner = "logos-blockchain";
    repo = "logos-execution-zone";
    rev = "27360cb7d6ccb2bfbcca7d171bab8a3938490264";
    sha256 = "sha256-YrA4tAu1G7drJaaG4c7xX72yBMMoSHmbTNS2UYqtxFY=";
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
