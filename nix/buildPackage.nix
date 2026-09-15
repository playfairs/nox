{
  pkgs ? import <nixpkgs> { },
}:
pkgs.rustPlatform.buildRustPackage {
  pname = "nox";
  version = pkgs.lib.strings.trim (builtins.readFile ../VERSION);
  src = ../.;
  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "noml-0.1.0" = "sha256-dykjiEdvzIqlHhO5XyamJAG7ztvsZlsrBekj11iCo8c=";
    };
  };
  meta.mainProgram = "nox";
}
