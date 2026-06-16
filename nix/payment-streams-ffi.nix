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
  # LEZ main (491 merged) — https://github.com/logos-blockchain/logos-execution-zone/pull/491
  logosExecutionZoneSrc = fetchFromGitHub {
    owner = "logos-blockchain";
    repo = "logos-execution-zone";
    rev = "a8c81f5445166b22672a614b159a1c38a5907a65";
    sha256 = "sha256-xBrIY3W9P8wxAxJZKxcTkM+C2oktpaX+z5EY1VEMgUg=";
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
