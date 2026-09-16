{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
    in
    {
      devShells = builtins.listToAttrs (
        map (system: {
          name = system;
          value = nixpkgs.legacyPackages.${system}.mkShell {
            packages = with nixpkgs.legacyPackages.${system}; [
              nox
              rustc
              cargo
              rustfmt
            ];
          };
        }) systems
      );
    };
}
