{
  description = "The Nox Build System";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.treefmt-nix.url = "github:numtide/treefmt-nix";

  outputs =
    {
      self,
      nixpkgs,
      treefmt-nix,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems =
        function: nixpkgs.lib.genAttrs systems (system: function nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: {
        nox = pkgs.callPackage ./nix/buildPackage.nix { };
        default = pkgs.callPackage ./nix/buildPackage.nix { };
      });

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.system}.default}/bin/nox";
        };
      });

      formatter = forAllSystems (
        pkgs:
        import ./nix/formatter.nix {
          inherit pkgs;
          inputs = { inherit treefmt-nix; };
        }
      );

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            self.packages.${pkgs.system}.nox
            pkgs.rustc
            pkgs.cargo
            pkgs.rustfmt
            pkgs.clang
            pkgs.nixfmt
          ];
        };
      });

      checks = forAllSystems (pkgs: {
        package = self.packages.${pkgs.system}.default;
      });
    };
}
