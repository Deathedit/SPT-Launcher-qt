# spt-launcher-qt: Rust + QML (cxx-qt) launcher & installer front end.
#
# It talks to the SPT server itself (profiles, bundles, patching, umu-run), so
# it replaces SPT.Launcher.Linux for daily use. Installing, Steam entries, Fika
# and Proton updates are delegated to the spt-additions CLI, which is put on
# its PATH together with spt-server / spt-launcher / umu-run.
{
  lib,
  rustPlatform,
  qt6,
  makeWrapper,
  librsvg,
  symlinkJoin,
  runCommand,
  google-fonts,
  ibm-plex,
  jetbrains-mono,
  hpatchz,
  _7zz,
  spt-additions, # or spt-additions-client
}:

let
  chakraPetch = google-fonts.override { fonts = [ "ChakraPetch" ]; };

  # cxx-qt-build asks qmake for the Qt prefix and expects every module (Qml,
  # Quick, ...) under it. In nixpkgs each module is its own store path, so give
  # it a merged environment whose qmake reports that merged prefix.
  qtEnv = qt6.env "spt-launcher-qt-env" [
    qt6.qtdeclarative
    qt6.qtwayland
    qt6.qtsvg
  ];

  # Only the faces the UI uses (keeps the closure small)
  fonts = runCommand "spt-launcher-qt-fonts" { } ''
    mkdir -p $out
    cp ${chakraPetch}/share/fonts/truetype/ChakraPetch-{Medium,SemiBold}.ttf $out/
    cp ${ibm-plex}/share/fonts/opentype/IBMPlexSans-{Regular,Medium,SemiBold}.otf $out/
    cp ${jetbrains-mono}/share/fonts/truetype/JetBrainsMono-{Regular,Medium}.ttf $out/
  '';

  unwrapped = rustPlatform.buildRustPackage {
    pname = "spt-launcher-qt-unwrapped";
    version = "0.1.0";

    src = lib.fileset.toSource {
      root = ../launcher;
      fileset = lib.fileset.unions [
        ../launcher/Cargo.toml
        ../launcher/Cargo.lock
        ../launcher/crates
      ];
    };
    cargoLock.lockFile = ../launcher/Cargo.lock;

    nativeBuildInputs = [
      qtEnv
      qt6.wrapQtAppsHook
      makeWrapper
      librsvg
    ];
    buildInputs = [
      qt6.qtbase
      qt6.qtdeclarative
      qt6.qtwayland
      qt6.qtsvg
    ];

    # qtbase's setup hook exports QMAKE=<qtbase>/bin/qmake; override it after the hooks ran
    preBuild = ''
      export QMAKE=${qtEnv}/bin/qmake
    '';

    postInstall = ''
      install -Dm644 ${../launcher/assets/spt-launcher-qt.desktop} $out/share/applications/spt-launcher-qt.desktop
      icon=${../launcher/crates/spt-launcher-qt/icons/spt-launcher-qt.svg}
      install -Dm644 $icon $out/share/icons/hicolor/scalable/apps/spt-launcher-qt.svg
      for s in 16 24 32 48 64 128 256; do
        mkdir -p $out/share/icons/hicolor/''${s}x''${s}/apps
        rsvg-convert -w $s -h $s $icon -o $out/share/icons/hicolor/''${s}x''${s}/apps/spt-launcher-qt.png
      done
    '';

    # wrapQtAppsHook adds QT_PLUGIN_PATH / QML_IMPORT_PATH
    qtWrapperArgs = [
      "--set-default SPT_LAUNCHER_FONTS ${fonts}"
      "--prefix PATH : ${
        lib.makeBinPath [
          spt-additions
          hpatchz
          _7zz # mod archives
        ]
      }"
    ];

    meta = {
      description = "SPT launcher & installer (Rust + QML) for NixOS";
      license = lib.licenses.mit;
      platforms = [ "x86_64-linux" ];
      mainProgram = "spt-launcher-qt";
    };
  };
in
# Installing this one package gives the GUI plus the whole spt-additions set
symlinkJoin {
  name = "spt-launcher-qt-0.1.0";
  paths = [
    unwrapped
    spt-additions
  ];
  passthru = { inherit unwrapped fonts qtEnv; };
  inherit (unwrapped) meta;
}
