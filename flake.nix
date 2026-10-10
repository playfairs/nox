{
  description = "Flake for Nox";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/26.05";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    self.submodules = true;
  };

  outputs =
    {
      flake-utils,
      nixpkgs,
      self,
      ...
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        nox = (pkgs.callPackage ./nix/buildPackage.nix { }) inputs;
      in
      {
        devShells =
          let
            inherit (pkgs.callPackage ./nix/devshell-tools.nix { })
              mkComposedShell
              ;

            extraFrag = (pkgs.callPackage ./nix/extra.nix { }) self;
            rustFrag = (pkgs.callPackage ./nix/rustDevShell.nix { }) inputs;
          in
          mkComposedShell [ rustFrag ] { }
          // mkComposedShell [
            rustFrag
            extraFrag
          ] { nameOverride = "default"; };

        packages = {
          inherit nox;
          default = nox;
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/nox";
        };

        formatter = (pkgs.callPackage ./nix/formatter.nix { }) inputs;
      }
    );
}
