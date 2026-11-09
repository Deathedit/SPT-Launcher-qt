# The spt-additions installer, with every native tool it needs provided by Nix
# (no runtime downloads of jq/7zz/xxd/hpatchz/umu-run).
{
  lib,
  stdenvNoCC,
  makeWrapper,
  symlinkJoin,
  bash,
  coreutils,
  findutils,
  gnused,
  gnugrep,
  gawk,
  gnutar,
  gzip,
  xz,
  curl,
  cacert,
  jq,
  xxd,
  _7zz,
  hpatchz,
  umu-launcher,
  procps,
  util-linux,
  xdg-user-dirs,
  which,
  cabextract,
  python3,
  dotnet,
  spt-server,
  spt-launcher,
  # false: client-only variant (no spt-server / ASP.NET runtime, defaults to --no-server)
  withServer ? true,
}:

let
  runtimeDeps = [
    bash
    coreutils
    findutils
    gnused
    gnugrep
    gawk
    gnutar
    gzip
    xz
    curl
    jq
    xxd
    _7zz
    hpatchz
    umu-launcher
    procps
    util-linux
    xdg-user-dirs
    which
    cabextract
    python3
    spt-launcher
  ]
  ++ lib.optionals withServer [
    dotnet
    spt-server
  ];

  script = stdenvNoCC.mkDerivation {
    pname = "spt-additions-script";
    version = "1.2.7-nix";

    src = lib.fileset.toSource {
      root = ../scripts;
      fileset = lib.fileset.unions [
        ../scripts/spt-additions
        ../scripts/metadata.conf
        ../scripts/steam-shortcut.py
      ];
    };

    nativeBuildInputs = [ makeWrapper ];
    dontConfigure = true;
    dontBuild = true;

    installPhase = ''
      runHook preInstall
      install -Dm755 spt-additions  $out/share/spt-additions/spt-additions
      install -Dm644 metadata.conf  $out/share/spt-additions/metadata.conf
      install -Dm755 steam-shortcut.py $out/share/spt-additions/steam-shortcut.py
      patchShebangs $out/share/spt-additions/spt-additions

      makeWrapper $out/share/spt-additions/spt-additions $out/bin/spt-additions \
        --prefix PATH : ${lib.makeBinPath runtimeDeps} \
        ${lib.optionalString withServer ''--set DOTNET_ROOT "${dotnet}/share/dotnet"''} \
        ${lib.optionalString (!withServer) "--set SPT_NO_SERVER 1"} \
        --set-default SSL_CERT_FILE "${cacert}/etc/ssl/certs/ca-bundle.crt" \
        --set SPT_NIX 1 \
        --set SPT_NIX_METADATA "$out/share/spt-additions/metadata.conf"
      runHook postInstall
    '';
  };
in
# Also expose spt-server, spt-launcher and umu-run, so installing this one
# package into a profile gives stable paths for desktop shortcuts and the SPT
# launcher's UmuPath setting.
symlinkJoin {
  name = "spt-additions${lib.optionalString (!withServer) "-client"}-1.2.7-nix";
  paths = [
    script
    spt-launcher
    umu-launcher
  ]
  ++ lib.optional withServer spt-server;
  passthru = { inherit script runtimeDeps; };
  meta = {
    description = "Guided SPT installer & management CLI (NixOS port of MadByteDE/SPT-Linux-Guide spt-additions)";
    homepage = "https://github.com/MadByteDE/SPT-Linux-Guide";
    license = lib.licenses.mit;
    platforms = [ "x86_64-linux" ];
    mainProgram = "spt-additions";
  };
}
