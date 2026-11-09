# Non-flake shell: `nix-shell`
{
  pkgs ? import <nixpkgs> { },
}:
(import ./default.nix { inherit pkgs; }).shell
