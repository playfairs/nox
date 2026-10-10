{
  typos,
  reuse,
  stdenv,
  renovate,
  callPackage,
}:
{
  packages,
  ...
}:
let
  inherit (callPackage ./devshell-tools.nix { }) mkFragment;
in
mkFragment {
  name = "extra";
  buildInputs = [
    reuse
    typos
    renovate
    packages.${stdenv.hostPlatform.system}.nox
  ];
}
