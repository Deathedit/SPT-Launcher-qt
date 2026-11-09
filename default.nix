# Non-flake entry point:
#   nix-build -A spt-additions
#   nix-env -if . -A spt-additions
{
  pkgs ? import <nixpkgs> { },
}:
import ./nix/packages.nix { inherit pkgs; }
