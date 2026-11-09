{
  description = "SPT (Single Player Tarkov) on NixOS: spt-additions installer, FHS-wrapped server & launcher";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      spt = import ./nix/packages.nix { inherit pkgs; };
      mkApp = drv: name: {
        type = "app";
        program = "${drv}/bin/${name}";
        meta.description = drv.meta.description or name;
      };
    in
    {
      packages.${system} = {
        inherit (spt)
          default
          spt-additions
          spt-additions-client
          spt-server
          spt-launcher
          spt-fhs
          hpatchz
          spt-launcher-qt
          spt-launcher-qt-client
          ;
      };

      apps.${system} = {
        default = mkApp spt.spt-additions "spt-additions";
        spt-additions = mkApp spt.spt-additions "spt-additions";
        spt-additions-client = mkApp spt.spt-additions-client "spt-additions";
        spt-server = mkApp spt.spt-server "spt-server";
        spt-launcher = mkApp spt.spt-launcher "spt-launcher";
        spt-fhs = mkApp spt.spt-fhs "spt-fhs";
        spt-launcher-qt = mkApp spt.spt-launcher-qt "spt-launcher-qt";
      };

      devShells.${system} = {
        default = spt.shell;
        launcher = spt.launcherShell;
      };

      overlays.default =
        final: _prev:
        let
          s = import ./nix/packages.nix { pkgs = final; };
        in
        {
          inherit (s)
            spt-additions
            spt-additions-client
            spt-server
            spt-launcher
            hpatchz
            spt-launcher-qt
            spt-launcher-qt-client
            ;
        };

      formatter.${system} = pkgs.nixfmt;
    };
}
