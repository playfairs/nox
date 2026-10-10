{
  stdenv,
  callPackage,
}:
{
  fenix,
  ...
}:
let
  inherit (callPackage ./devshell-tools.nix { }) mkFragment;
  toolchain =
    with fenix.packages.${stdenv.system};
    combine [
      latest.toolchain
    ];
in
mkFragment {
  name = "rust";
  buildInputs = [
    toolchain
  ];
}
