{
  pkgs,
  inputs,
}:
(inputs.treefmt-nix.lib.evalModule pkgs (_: {
  projectRootFile = ".git/config";
  programs = {
    nixfmt.enable = true;
    nixf-diagnose.enable = true;
    taplo.enable = true;
    rustfmt.enable = true;
  };
})).config.build.wrapper
