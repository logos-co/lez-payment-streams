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
  # Pinned to PR 429 (wallet FFI arbitrary public transactions):
  # https://github.com/logos-blockchain/logos-execution-zone/pull/429
  logosExecutionZoneSrc = fetchFromGitHub {
    owner = "logos-blockchain";
    repo = "logos-execution-zone";
    rev = "6721d8d96e71566f072bab2ededcf56d29b002b0";
    sha256 = "sha256-t0SsUY2+gusYfvTZP1yUORIhlDiQWagV6pUUwCplEew=";
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
