# FHS environment for the native SPT binaries. These are prebuilt, dynamically
# linked executables that expect a regular FHS distro, an ASP.NET Core runtime
# and webkit2gtk-4.1 — none of which exist at the usual paths on NixOS.
#
#   SPT 4.1+ (latest):  SPT_Runtime/SPT.Server.Linux + SPT_Runtime/SPT.Launcher.Linux  (ASP.NET 10)
#   SPT 4.0 (legacy):   SPT/SPT.Server.Linux (ASP.NET 9) + SPT/SPT.Launcher.exe via umu/Proton
{
  lib,
  buildFHSEnv,
  writeShellScript,
  writeShellScriptBin,
  glib-networking,
  umu-launcher,
  dotnet, # combined ASP.NET runtime(s), see packages.nix
}:

let
  dotnetRoot = "${dotnet}/share/dotnet";

  targetPkgs =
    pkgs: with pkgs; [
      # .NET apphost + runtime dependencies
      dotnet
      icu
      openssl
      zlib
      krb5
      stdenv.cc.cc.lib

      # SPT.Launcher.Linux (GUI / webview)
      webkitgtk_4_1
      gtk3
      glib
      glib-networking
      libsoup_3
      gdk-pixbuf
      cairo
      pango
      harfbuzz
      at-spi2-atk
      fontconfig
      freetype
      libGL
      mesa
      libxkbcommon
      wayland
      libx11
      libxcursor
      libxrandr
      libxi
      libice
      libsm
      dbus
      gsettings-desktop-schemas
      libnotify # Photino.Native (SPT.Launcher.Linux)
      hicolor-icon-theme
      adwaita-icon-theme

      # The launcher starts the game through umu-run
      umu-launcher

      # Misc tools the wrappers use
      bash
      coreutils
      gnugrep
      gnused
      procps
    ];

  profile = ''
    export DOTNET_ROOT="${dotnetRoot}"
    # nix-shell's private TMPDIR does not exist inside umu's container
    unset TMP TEMP TEMPDIR
    export TMPDIR=/tmp
    export GIO_EXTRA_MODULES="${glib-networking}/lib/gio/modules''${GIO_EXTRA_MODULES:+:$GIO_EXTRA_MODULES}"
    # WebKitGTK's DMA-BUF renderer often shows a blank window inside bwrap
    export WEBKIT_DISABLE_DMABUF_RENDERER="''${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
  '';

  # Resolve the SPT install directory:
  #   1. first argument (if it is a directory)
  #   2. $SPT_PATH
  #   3. spt-path= from spt-additions' app.conf
  #   4. ~/Games/SPT
  resolveSptPath = cmd: ''
    if [[ "$(id -u)" -eq 0 ]]; then
        echo "This script is not supposed to be run as root!" >&2
        exit 1
    fi

    spt_path=""
    if [[ -n "''${1:-}" && -d "$1" ]]; then
        spt_path="$1"; shift
    elif [[ -n "''${SPT_PATH:-}" ]]; then
        spt_path="$SPT_PATH"
    else
        conf="''${XDG_CONFIG_HOME:-$HOME/.config}/spt-additions/app.conf"
        if [[ -f "$conf" ]]; then
            spt_path="$(grep '^spt-path=' "$conf" | tail -1 | cut -d= -f2- | tr -d '"')"
        fi
    fi
    spt_path="''${spt_path:-$HOME/Games/SPT}"
    spt_path="''${spt_path%/}"

    if [[ -d "$spt_path/SPT_Runtime" ]]; then runtime_dir="$spt_path/SPT_Runtime"
    elif [[ -d "$spt_path/SPT" ]]; then runtime_dir="$spt_path/SPT"   # SPT 4.0 (legacy)
    else
        echo "ERROR: No SPT installation found in \"$spt_path\"" >&2
        echo "  ► Install SPT with 'spt-additions' first," >&2
        echo "  ► or pass the SPT directory, e.g.: ${cmd} ~/Games/SPT" >&2
        exit 1
    fi
  '';

  mkSptEnv =
    { name, runScript }:
    buildFHSEnv {
      inherit
        name
        targetPkgs
        profile
        runScript
        ;
      # SPT has no 32-bit parts; Proton/umu bring their own runtime.
      multiArch = false;
      dieWithParent = true;
      meta.platforms = [ "x86_64-linux" ];
    };

  serverScript = writeShellScript "spt-server-run" ''
    ${resolveSptPath "spt-server"}
    if pidof SPT.Server.Linux &>/dev/null; then
        echo "ERROR: Another instance of the SPT server is already running" >&2
        exit 1
    fi
    cd "$runtime_dir" || exit 1
    exec ./SPT.Server.Linux "$@"
  '';

  launcherScript = writeShellScript "spt-launcher-run" ''
    ${resolveSptPath "spt-launcher"}
    cd "$runtime_dir" || exit 1
    if [[ -x ./SPT.Launcher.Linux ]]; then
        exec ./SPT.Launcher.Linux "$@"
    elif [[ -f ./SPT.Launcher.exe ]]; then
        # SPT 4.0 (legacy): Windows launcher through umu/Proton
        export PROTONPATH="''${PROTONPATH:-GE-Proton}"
        export WINEPREFIX="''${WINEPREFIX:-$HOME/Games/SPT-Prefix}"
        export PROTON_USE_XALIA="''${PROTON_USE_XALIA:-0}"
        export PROTON_MEDIA_USE_GST="''${PROTON_MEDIA_USE_GST:-1}"
        unset DOTNET_ROOT DOTNET_BUNDLE_EXTRACT_BASE_DIR
        exec umu-run SPT.Launcher.exe "$@"
    fi
    echo "ERROR: No SPT launcher found in \"$runtime_dir\"" >&2
    exit 1
  '';

  serverFHS = mkSptEnv {
    name = "spt-server-fhs";
    runScript = serverScript;
  };

  # spt-server: run in the current terminal, or open a terminal emulator when
  # started without a TTY (e.g. from the application menu) — same behaviour as
  # upstream's server.sh. SPT_SERVER_NO_TERMINAL=1 (set by spt-launcher-qt, which
  # captures the output itself) always runs it in place.
  spt-server = writeShellScriptBin "spt-server" ''
    TERMINALS=( "alacritty" "ghostty" "foot" "terminator" "ptyxis" "cosmic-terminal"
                "kgx" "konsole" "gnome-terminal" "xfce4-terminal" "kitty" "wezterm" "xterm" )
    if [[ -t 0 || "''${SPT_SERVER_NO_TERMINAL:-}" == 1 ]]; then
        exec "${serverFHS}/bin/spt-server-fhs" "$@"
    fi
    for term in "''${TERMINALS[@]}"; do
        command -v "$term" &>/dev/null || continue
        case "$term" in
            gnome-terminal|kgx|ptyxis) exec "$term" -- "${serverFHS}/bin/spt-server-fhs" "$@" ;;
            *) exec "$term" -e "${serverFHS}/bin/spt-server-fhs" "$@" ;;
        esac
    done
    echo "ERROR: Failed to find a terminal emulator" >&2
    exit 1
  '';
in
{
  inherit spt-server dotnetRoot;

  spt-launcher = mkSptEnv {
    name = "spt-launcher";
    runScript = launcherScript;
  };

  # Interactive shell inside the same FHS env, for debugging missing libraries:
  #   nix run .#spt-fhs
  #   nix run .#spt-fhs -- -c 'ldd ~/Games/SPT/SPT_Runtime/SPT.Launcher.Linux'
  spt-fhs = mkSptEnv {
    name = "spt-fhs";
    runScript = "bash";
  };
}
