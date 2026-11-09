# Glue: builds every package of this repo from a given nixpkgs.
#   withLegacy: also ship ASP.NET 9 for the SPT 4.0 ("legacy" install mode) server
{
  pkgs,
  withLegacy ? true,
}:

let
  inherit (pkgs) callPackage;

  dotnet = pkgs.dotnetCorePackages.combinePackages (
    [ pkgs.dotnetCorePackages.aspnetcore_10_0 ]
    ++ pkgs.lib.optional withLegacy pkgs.dotnetCorePackages.aspnetcore_9_0
  );

  hpatchz = callPackage ./hpatchz.nix { };
  runtime = callPackage ./spt-runtime.nix { inherit dotnet; };

  spt-additions = callPackage ./spt-additions.nix {
    inherit hpatchz dotnet;
    inherit (runtime) spt-server spt-launcher;
  };

  # Client-only: no local server (e.g. for joining a remote/Fika host)
  spt-additions-client = spt-additions.override { withServer = false; };

  spt-launcher-qt = callPackage ./spt-launcher-qt.nix { inherit hpatchz spt-additions; };
  spt-launcher-qt-client = spt-launcher-qt.override { spt-additions = spt-additions-client; };
in
{
  inherit
    hpatchz
    spt-additions
    spt-additions-client
    spt-launcher-qt
    spt-launcher-qt-client
    ;
  inherit (runtime) spt-server spt-launcher spt-fhs;
  default = spt-launcher-qt;

  shell = pkgs.mkShell {
    # runtimeDeps too, so ./scripts/spt-additions can be run directly while hacking
    packages = [
      spt-additions
      runtime.spt-fhs
      pkgs.shellcheck
    ]
    ++ spt-additions.runtimeDeps;
    SPT_NIX_METADATA = toString ../scripts/metadata.conf;
    DOTNET_ROOT = runtime.dotnetRoot;
    shellHook = ''
      echo "SPT NixOS shell"
      echo "  spt-additions          guided installer / management CLI"
      echo "  spt-server             run the SPT server (FHS env)"
      echo "  spt-launcher           run the SPT launcher (FHS env)"
      echo "  spt-fhs                debug shell inside the FHS env"
    '';
  };

  # Rust + QML launcher development: nix develop .#launcher
  launcherShell = pkgs.mkShell {
    inputsFrom = [ spt-launcher-qt.unwrapped ];
    # _FORTIFY_SOURCE warns on every C++ file in unoptimised (debug) builds
    hardeningDisable = [ "fortify" ];
    packages = [
      spt-additions
      hpatchz
      pkgs._7zz
      pkgs.clippy
      pkgs.rustfmt
      pkgs.rust-analyzer
    ];
    SPT_LAUNCHER_FONTS = spt-launcher-qt.fonts;
    QT_PLUGIN_PATH = pkgs.lib.makeSearchPath pkgs.qt6.qtbase.qtPluginPrefix [
      pkgs.qt6.qtbase
      pkgs.qt6.qtwayland
      pkgs.qt6.qtsvg
    ];
    QML_IMPORT_PATH = pkgs.lib.makeSearchPath pkgs.qt6.qtbase.qtQmlPrefix [ pkgs.qt6.qtdeclarative ];
    # This repo is not a git checkout, so the flake copies the whole directory to
    # the store on every evaluation: keep cargo's target/ out of it.
    shellHook = ''
      export CARGO_TARGET_DIR="''${XDG_CACHE_HOME:-$HOME/.cache}/spt-launcher-qt/target"
      # qtbase's setup hook points QMAKE at qtbase alone; cxx-qt needs the merged env
      export QMAKE=${spt-launcher-qt.qtEnv}/bin/qmake
      echo "spt-launcher-qt dev shell: cd launcher && cargo run"
    '';
  };
}
