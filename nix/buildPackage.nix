{
  stdenv,
  pkgs,
}:
{
  fenix,
  crane,
  ...
}:
let
  toolchain =
    with fenix.packages.${stdenv.hostPlatform.system};
    combine [
      latest.toolchain
    ];
  craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
  src = ../.;
in
craneLib.buildPackage {
  inherit src;
  strictDeps = true;
} 
