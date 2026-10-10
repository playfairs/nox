{
  pkgs,
}:
{
  treefmt-nix,
  ...
}:
(treefmt-nix.lib.evalModule pkgs (_: {
  projectRootFile = "flake.nix";
  programs = {
    nixfmt.enable = true;
    nixf-diagnose.enable = true;
    taplo.enable = true;
    rustfmt.enable = true;
  };
})).config.build.wrapper
