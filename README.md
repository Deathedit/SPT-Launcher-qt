# SPT on NixOS

A NixOS port of the `spt-additions` CLI installer from
[MadByteDE/SPT-Linux-Guide](https://github.com/MadByteDE/SPT-Linux-Guide), packaged as a Nix flake
(with a `shell.nix` for non-flake users).

> [!IMPORTANT]
> This is a community port and is **NOT** officially supported by the SPT developers or the
> SPT-Linux-Guide maintainers. Don't report NixOS-specific issues to them.

## What's different from upstream

| Upstream (FHS distros)                                              | This port                                                                 |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Downloads `jq`, `7zzs`, `xxd`, `hpatchz`, `umu-run` at runtime      | All tools come from Nix (`jq`, `_7zz`, `xxd`, HDiffPatch, `umu-launcher`) |
| Requires system `aspnetcore-runtime-10.0` + `webkit2gtk-4.1`        | Provided inside a `buildFHSEnv` (ASP.NET 10 + 9 for legacy SPT 4.0)       |
| `SPT.Server.Linux` / `SPT.Launcher.Linux` run directly              | Run via the `spt-server` / `spt-launcher` FHS wrappers                    |
| Downloads `server.sh` / `launcher.sh` & `metadata.conf` from GitHub | Generated locally / shipped with the package                              |
| `self-install` / `self-update`                                      | Managed by Nix (`nix profile`, `nix flake update`)                        |

Everything else is the same: guided install, Steam-version detection, GE-Proton download
(`~/.steam/steam/compatibilitytools.d`), prefix workarounds, downpatching, Fika, shortcuts,
`run` / `update` / `uninstall` / `clean`.

## Requirements

- NixOS (x86_64) with flakes enabled (`nix.settings.experimental-features = [ "nix-command" "flakes" ];`)
- Working GPU / Vulkan drivers (`hardware.graphics.enable = true;`)
- ~150 GB free disk space
- Optional: Steam, if you own the Steam version of the game

## Quick start

```bash
git clone <this repo> ~/SPT-NixOS-Guide
cd ~/SPT-NixOS-Guide

# Recommended: install into your profile, so desktop shortcuts keep working
nix profile add .#spt-additions
spt-additions                 # guided installer

# One-off (shortcuts break after garbage collection, the script warns about this)
nix run .#spt-additions
```

After installing you get:

| Command         | What it does                                       |
| --------------- | -------------------------------------------------- |
| `spt-additions` | Guided installer & management CLI (`-h` for help)  |
| `spt-server`    | Runs the SPT server (opens a terminal if needed)   |
| `spt-launcher`  | Runs the SPT launcher                              |
| `umu-run`       | Used by the launcher / shortcuts to start the game |

Both `spt-server` and `spt-launcher` take an optional SPT directory (`spt-server ~/Games/SPT`). They
fall back to `$SPT_PATH`, then to `spt-path` in `~/.config/spt-additions/app.conf`, then to `~/Games/SPT`.

## SPT Launcher (Rust + Qt)

`spt-launcher-qt` is a native launcher and installer that replaces `SPT.Launcher.Linux`. It talks to the
SPT server directly (same API as the official launcher) and runs the game through `umu-run`.

```bash
nix profile add .#spt-launcher-qt          # or: nix-env -if . -A spt-launcher-qt
spt-launcher-qt                            # also in the app menu as "SPT Launcher"

nix profile add .#spt-launcher-qt-client   # client-only variant (no spt-server)
```

The package includes `spt-additions`, so you don't need to install it separately.

| Page     | What it does                                                                                    |
| -------- | ----------------------------------------------------------------------------------------------- |
| Play     | Connect to the local server or a remote/Fika host, manage profiles, launch                       |
| Server   | Start/stop/restart the local `spt-server`, live console with filters and stdin                  |
| Install  | Install SPT from your Steam copy (runs `spt-additions` in the background), updates, patching  |
| Proton   | Pick the Proton build, download the latest GE-Proton                                            |
| Settings | Paths, launch options, gamemode, Steam entry, shortcuts, Fika, uninstall                        |

Launch does the same steps as the official launcher:
1. Checks `spt-core.dll` against the server version.
2. Cleans up BattlEye and other leftover files.
3. Applies the downpatch deltas (`hpatchz`).
4. Downloads mod bundles into `SPT_Runtime/user/cache/bundles`.
5. Starts `umu-run EscapeFromTarkov.exe` with `WINEPREFIX` and `PROTONPATH`.

Settings are shared with the official launcher (`SPT_Runtime/user/Launcher/LauncherSettings.json`) and
`app.conf`. The launcher's own UI state lives in `~/.config/spt-additions/launcher-qt.json`.

- **Tray:** a tray icon (StatusNotifierItem: KDE, GNOME with AppIndicator, waybar, noctalia and others)
  with Show/Hide, Play, Stop game and Quit. When it's on, closing the window hides the launcher to the
  tray. *Hide to tray while playing* hides the window when the game starts and brings it back when the
  game exits. Both options are in *Settings → Integration*.
- **Fallback:** if Play breaks after an SPT update, use *Settings → Fallback → Official SPT launcher*.
- **Installer limits:** the in-app installer needs a Steam copy of EFT. For a BSG-launcher copy, click
  *Guided installer (terminal)*, which opens the interactive `spt-additions` in a terminal.

**Development:**
```bash
nix develop .#launcher
cd launcher && cargo run
```
The build output goes to `~/.cache/spt-launcher-qt/target`. Environment hooks for testing:
- `SPT_LAUNCHER_VIEW=server` opens that page on start.
- `SPT_LAUNCHER_SCREENSHOT=/tmp/x.png` saves a screenshot and quits; `SPT_LAUNCHER_SCREENSHOT_DELAY=ms` sets the delay.
- `SPT_LAUNCHER_ECHO_LOG=1` copies the logs to stderr.
- `SPT_LAUNCHER_AUTOLAUNCH=1` launches as soon as it connects.
- `QT_FORCE_STDERR_LOGGING=1` shows QML errors on stderr.

## Client-only install (no local server)

If you only want to **join** a server, e.g. a friend's Fika host or a dedicated/headless server, skip
the local server setup:

```bash
# Option 1: the client-only package (no spt-server / ASP.NET runtime at all)
nix profile add .#spt-additions-client
spt-additions

# Option 2: the normal package with a switch (remembered in app.conf)
spt-additions --no-server            # guided install, client only
spt-additions --no-server install spt
spt-additions --with-server install spt   # switch back
```

The guided installer also asks *"Do you want to set up the local SPT server?"*.

In client-only mode:

- the ASP.NET runtime check is skipped
- no `server.sh` and no "SPT - Server" shortcut are created
- `install fika` installs only the Fika client plugin, not the Fika server mod

> [!NOTE]
> The SPT archive still contains the server files, and the launcher lives in the same `SPT_Runtime`
> directory. They just aren't set up or launched.

## Add SPT to Steam (non-Steam game)

```bash
spt-additions steam              # adds "SPT - Launcher" (+ "SPT - Server" unless --no-server)
spt-additions steam launcher     # only the launcher
spt-additions steam remove       # remove the entries again
```

**Close Steam first** (Steam → Exit). Steam rewrites its shortcuts when it exits, so the script
refuses to run while it's open.

- **Which account:** entries go to the most recently logged-in Steam account. Use
  `SPT_STEAM_USER=all` for every account, or `SPT_STEAM_USER=<account id>` for one
  (the folder name under `~/.steam/steam/userdata/`).
- **Artwork:** the icon and portrait cover are set automatically, and the entries are tagged `SPT`.
- **Backup:** `shortcuts.vdf` is backed up to `shortcuts.vdf.spt-backup` before every change.
- **When it's offered:** the guided installer asks at the end, and `uninstall spt` removes the
  entries.
- **How it runs:** the entries point at `launcher.sh` / `server.sh`, which drop Steam's injected
  `LD_PRELOAD` / `LD_LIBRARY_PATH` and start the Nix FHS wrappers. Leave Steam's
  "Compatibility" setting for these entries **off**, since they are native scripts.

## NixOS / home-manager configuration

```nix
# flake.nix of your system
inputs.spt-nixos.url = "path:/home/<you>/SPT-NixOS-Guide";   # or a git URL

# in a module
{ inputs, pkgs, ... }: {
  nixpkgs.overlays = [ inputs.spt-nixos.overlays.default ];

  environment.systemPackages = [ pkgs.spt-launcher-qt ]; # or spt-launcher-qt-client / spt-additions
  # home-manager: home.packages = [ pkgs.spt-additions ];
}
```

## Without flakes

```bash
nix-shell                                  # shell with spt-additions & friends
nix-env -if . -A spt-additions             # install into your profile
nix-build -A spt-additions-client          # build the client-only variant
```

The SPT 4.0 (legacy) server needs ASP.NET 9. To drop it, use
`import ./nix/packages.nix { pkgs = ...; withLegacy = false; }`.

## Troubleshooting

- **Logs:** `~/.cache/spt-additions/installer.log`
- **Missing libraries for the native binaries:** open a shell in the same FHS env:
  ```bash
  nix run .#spt-fhs
  # inside:
  ldd ~/Games/SPT/SPT_Runtime/SPT.Launcher.Linux | grep "not found"
  ```
  Add missing libs to `targetPkgs` in `nix/spt-runtime.nix`.
- **Blank launcher window:** the wrapper sets `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Try
  `WEBKIT_DISABLE_DMABUF_RENDERER=0 spt-launcher` or `GDK_BACKEND=x11 spt-launcher`.
- **"is only available from the Nix store" warning:** you're running via `nix run`. Install the
  package into a profile so shortcuts and `LauncherSettings.json` (`UmuPath`) keep working.
- **Updating:** `nix flake update && nix profile upgrade spt-additions`. Updating SPT itself still
  works with `spt-additions update spt`.

## Repository layout

```
flake.nix / default.nix / shell.nix   entry points
nix/packages.nix                      builds everything (withLegacy flag)
nix/spt-additions.nix                 wrapped installer (withServer flag → spt-additions-client)
nix/spt-runtime.nix                   buildFHSEnv: spt-server, spt-launcher, spt-fhs
nix/hpatchz.nix                       HDiffPatch 5.1.3 (not in nixpkgs)
nix/spt-launcher-qt.nix               Rust + Qt launcher package (+ bundled fonts)
launcher/crates/spt-core              launcher logic: SPT API, bundles, patching, launch (no Qt)
launcher/crates/spt-launcher-qt       cxx-qt bridge + QML UI
scripts/spt-additions                 the installer (based on upstream 1.2.7)
scripts/metadata.conf                 URLs used by the installer
```

## Credits

All the hard work is by **MadByte** and the
[SPT-Linux-Guide contributors](https://github.com/MadByteDE/SPT-Linux-Guide#credits). This port only
adapts it to NixOS. Thanks to the NixOS folks in
[PR #14](https://github.com/MadByteDE/SPT-Linux-Guide/pull/14) for the groundwork.
MIT licensed, see [LICENSE](LICENSE).

Help & community: [SPT Discord](https://discord.com/invite/Xn9msqQZan).
