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
  # LEZ main at PR 510 merge (program deploy + test ELF FFI).
  logosExecutionZoneSrc = fetchFromGitHub {
    owner = "logos-blockchain";
    repo = "logos-execution-zone";
    rev = "62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60";
    sha256 = "sha256-lhKiGOWisFxEs3tiZ6/mO+4d78D9WolHh3BQsMexFoQ=";
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
