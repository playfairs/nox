{
  pkgs ? import <nixpkgs> { },
}:
pkgs.rustPlatform.buildRustPackage {
  pname = "nox";
  version = pkgs.lib.strings.trim (builtins.readFile ../VERSION);
  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;
  meta.mainProgram = "nox";
}
