# Non-flake entry point:
#   nix-build                       (spt-launcher-qt)
#   nix-env -if . -A spt-launcher-qt
{
  pkgs ? import <nixpkgs> { },
}:
import ./nix/packages.nix { inherit pkgs; }
