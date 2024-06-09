{
  description = "Blahaj";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/";

  outputs =
    { self, nixpkgs }:
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
      packages = forAllSystems (pkgs: rec {
        default = blahaj;
        blahaj = pkgs.callPackage ./default.nix { rev = self.dirtyRev or self.rev; };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.callPackage ./shell.nix { };
      });
    };
}
