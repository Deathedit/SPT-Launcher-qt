# SPT-Launcher-qt

A native **Rust + Qt (QML)** launcher and installer for [SPT (Single Player Tarkov)](https://sp-tarkov.com)
on Linux, packaged as a Nix flake for **NixOS**. It replaces `SPT.Launcher.Linux` and comes with a
Nix port of the `spt-additions` installer from
[MadByteDE/SPT-Linux-Guide](https://github.com/MadByteDE/SPT-Linux-Guide).

> [!IMPORTANT]
> This is a community project and is **NOT** officially supported by the SPT developers or the
> SPT-Linux-Guide maintainers. Report issues here:
> [Deathedit/SPT-Launcher-qt/issues](https://github.com/Deathedit/SPT-Launcher-qt/issues).

## Features

- **Play:** connect to your local server or a remote / Fika host, create, wipe and remove profiles,
  and launch in one click.
- **Launches like the official launcher:**
  1. Checks the client version against the server.
  2. Cleans up BattlEye and other leftover files.
  3. Applies the downpatch deltas.
  4. Syncs mod bundles.
  5. Starts the game through `umu-run` and Proton.
- **Server:** start, stop and restart the local `spt-server`, with a live console (filters, stdin).
- **Install:** installs SPT from your Steam copy of EFT (runs `spt-additions` in the background),
  plus updates and patching.
- **Proton:** pick a Proton build or download the latest GE-Proton.
- **Settings:** launch options, gamemode, Steam non-Steam entry, desktop shortcuts, Fika, uninstall.
- **Tray:** a tray icon with Play and Stop game, close to tray, and the window hides while you play.
- **Nix-native:** every tool comes from Nix, the native SPT binaries run in a `buildFHSEnv`, and
  nothing is downloaded at runtime except SPT, mods and GE-Proton.

## Requirements

- NixOS (x86_64) with flakes enabled (`nix.settings.experimental-features = [ "nix-command" "flakes" ];`)
- Working GPU / Vulkan drivers (`hardware.graphics.enable = true;`)
- ~150 GB free disk space
- A copy of Escape from Tarkov. The in-app installer needs the **Steam** version; for the BSG
  launcher version, use the guided terminal installer (see below).
- Optional: `programs.gamemode.enable = true;` for the gamemode option

## Quick start

```bash
# Install (recommended: shortcuts and Steam entries need a stable profile path)
nix profile add github:Deathedit/SPT-Launcher-qt
spt-launcher-qt                 # also in the app menu as "SPT Launcher"

# Or just try it
nix run github:Deathedit/SPT-Launcher-qt
```

On first start the launcher opens the **Install** page. Pick a folder, then **Start install**. When
it's done, go to **Play**, connect, pick or create a profile, and hit **Launch**.

The package also contains:

| Command           | What it does                                          |
| ----------------- | ----------------------------------------------------- |
| `spt-launcher-qt` | The launcher (GUI)                                    |
| `spt-additions`   | Guided installer & management CLI (`-h` for help)     |
| `spt-server`      | Runs the SPT server (opens a terminal if needed)      |
| `spt-launcher`    | The official `SPT.Launcher.Linux`, as a fallback      |
| `umu-run`         | Used to start the game                                |

`spt-server` and `spt-launcher` take an optional SPT directory (`spt-server ~/Games/SPT`). They
fall back to `$SPT_PATH`, then to `spt-path` in `~/.config/spt-additions/app.conf`, then to `~/Games/SPT`.

### From a local clone

```bash
git clone https://github.com/Deathedit/SPT-Launcher-qt.git
cd SPT-Launcher-qt
nix profile add .#spt-launcher-qt     # flakes
nix-env -if . -A spt-launcher-qt      # without flakes
```

## Packages

| Flake output             | Contents                                                     |
| ------------------------ | ------------------------------------------------------------ |
| `default`                | same as `spt-launcher-qt`                                    |
| `spt-launcher-qt`        | launcher + `spt-additions`, `spt-server`, `spt-launcher`     |
| `spt-launcher-qt-client` | client-only launcher (no `spt-server` / ASP.NET runtime)     |
| `spt-additions`          | only the CLI installer + server/launcher wrappers            |
| `spt-additions-client`   | client-only CLI                                              |
| `spt-fhs`                | debug shell in the FHS environment of the native binaries    |
| `hpatchz`                | HDiffPatch 5.1.3 (not in nixpkgs)                            |

## NixOS / home-manager configuration

```nix
# flake.nix of your system
inputs.spt-launcher-qt.url = "github:Deathedit/SPT-Launcher-qt";

# in a module
{ inputs, pkgs, ... }: {
  nixpkgs.overlays = [ inputs.spt-launcher-qt.overlays.default ];

  environment.systemPackages = [ pkgs.spt-launcher-qt ];  # or pkgs.spt-launcher-qt-client
  # home-manager: home.packages = [ pkgs.spt-launcher-qt ];
}
```

## Using the launcher

Settings are shared with the official launcher (`SPT_Runtime/user/Launcher/LauncherSettings.json`)
and with `~/.config/spt-additions/app.conf`. The launcher's own UI state (target host, last profile,
tray options) lives in `~/.config/spt-additions/launcher-qt.json`.

- **Remote / Fika host:** on **Play**, switch to *Remote / Fika host* and enter `host:port`.
- **Tray:** works with StatusNotifierItem tray hosts (KDE, GNOME with AppIndicator, waybar,
  noctalia and others). The tray menu has Show/Hide, Play, Stop game and Quit.
  - With the tray on, closing the window hides the launcher to the tray.
  - *Hide to tray while playing* hides the window when the game starts and brings it back when
    the game exits.
  - Both options are in *Settings → Integration*.
- **Fallback:** if Play breaks after an SPT update, use *Settings → Fallback → Official SPT launcher*.
- **BSG-launcher copy of EFT:** click *Install → Guided installer (terminal)*, which opens the
  interactive `spt-additions` in a terminal.

## Client-only (no local server)

If you only want to **join** a server, e.g. a friend's Fika host or a dedicated/headless server:

```bash
nix profile add github:Deathedit/SPT-Launcher-qt#spt-launcher-qt-client
```

You can also use the normal package and turn off *Settings → Integration → Local server*, which
writes `server="0"` to `app.conf`. On the command line:

```bash
spt-additions --no-server            # guided install, client only
spt-additions --no-server install spt
spt-additions --with-server install spt   # switch back
```

In client-only mode:

- the ASP.NET runtime check is skipped
- no `server.sh` and no "SPT - Server" shortcut are created
- `install fika` installs only the Fika client plugin, not the Fika server mod

> [!NOTE]
> The SPT archive still contains the server files. They just aren't set up or launched.

## Add SPT to Steam (non-Steam game)

In the launcher: *Settings → Integration → Steam non-Steam game entry*. Or on the command line:

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
- **How it runs:** the entries point at `launcher.sh` / `server.sh`, which drop Steam's injected
  `LD_PRELOAD` / `LD_LIBRARY_PATH` and start the Nix FHS wrappers. Leave Steam's
  "Compatibility" setting for these entries **off**, since they are native scripts.

## The `spt-additions` port

`spt-additions` is the upstream CLI installer, adapted to NixOS. The launcher uses it for
installing, Steam entries, Fika and Proton updates.

| Upstream (FHS distros)                                              | This port                                                                 |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Downloads `jq`, `7zzs`, `xxd`, `hpatchz`, `umu-run` at runtime      | All tools come from Nix (`jq`, `_7zz`, `xxd`, HDiffPatch, `umu-launcher`) |
| Requires system `aspnetcore-runtime-10.0` + `webkit2gtk-4.1`        | Provided inside a `buildFHSEnv` (ASP.NET 10 + 9 for legacy SPT 4.0)       |
| `SPT.Server.Linux` / `SPT.Launcher.Linux` run directly              | Run via the `spt-server` / `spt-launcher` FHS wrappers                    |
| Downloads `server.sh` / `launcher.sh` & `metadata.conf` from GitHub | Generated locally / shipped with the package                              |
| `self-install` / `self-update`                                      | Managed by Nix (`nix profile`, `nix flake update`)                        |

Everything else is the same as upstream:
- guided install and Steam-version detection
- GE-Proton download to `~/.steam/steam/compatibilitytools.d`
- prefix workarounds, downpatching, Fika, shortcuts
- `run` / `update` / `uninstall` / `clean`

```bash
nix run github:Deathedit/SPT-Launcher-qt#spt-additions -- -h
```

The SPT 4.0 (legacy) server needs ASP.NET 9. To drop it, use
`import ./nix/packages.nix { pkgs = ...; withLegacy = false; }`.

## Troubleshooting

- **Logs:**
  - the launcher's log panels (Play, Server, Install)
  - `~/.cache/spt-additions/installer.log`
  - `QT_FORCE_STDERR_LOGGING=1 spt-launcher-qt` shows Qt/QML messages
- **`pressure-vessel-wrap ... ldconfig` / `unable to use parent for game drive` in the game log:**
  harmless umu/Proton messages on NixOS. The game still starts.
- **Missing libraries for the native binaries:** open a shell in the same FHS env:
  ```bash
  nix run github:Deathedit/SPT-Launcher-qt#spt-fhs
  # inside:
  ldd ~/Games/SPT/SPT_Runtime/SPT.Server.Linux | grep "not found"
  ```
  Add missing libs to `targetPkgs` in `nix/spt-runtime.nix`.
- **Blank official launcher window:** the wrapper sets `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Try
  `WEBKIT_DISABLE_DMABUF_RENDERER=0 spt-launcher` or `GDK_BACKEND=x11 spt-launcher`.
- **"is only available from the Nix store" warning:** you're running via `nix run`. Install the
  package into a profile so shortcuts and `LauncherSettings.json` (`UmuPath`) keep working.
- **Updating:**
  - the launcher: `nix profile upgrade spt-launcher-qt` (or `nix flake update` in your system flake)
  - SPT itself: *Install → Update SPT* or `spt-additions update spt`

## Development

```bash
git clone https://github.com/Deathedit/SPT-Launcher-qt.git && cd SPT-Launcher-qt
nix develop .#launcher
cd launcher && cargo run
```

The build output goes to `~/.cache/spt-launcher-qt/target`. `cargo test` and `cargo clippy` work in
the dev shell. Environment hooks for testing:

- `SPT_LAUNCHER_VIEW=server` opens that page on start.
- `SPT_LAUNCHER_SCREENSHOT=/tmp/x.png` saves a screenshot and quits; `SPT_LAUNCHER_SCREENSHOT_DELAY=ms` sets the delay.
- `SPT_LAUNCHER_ECHO_LOG=1` copies the logs to stderr.
- `SPT_LAUNCHER_AUTOLAUNCH=1` launches as soon as it connects.
- `QT_FORCE_STDERR_LOGGING=1` shows QML errors on stderr.

### Repository layout

```
flake.nix / default.nix / shell.nix   entry points
nix/packages.nix                      builds everything (withLegacy flag)
nix/spt-launcher-qt.nix               launcher package (+ bundled fonts, icons)
nix/spt-additions.nix                 wrapped installer (withServer flag → spt-additions-client)
nix/spt-runtime.nix                   buildFHSEnv: spt-server, spt-launcher, spt-fhs
nix/hpatchz.nix                       HDiffPatch 5.1.3 (not in nixpkgs)
launcher/crates/spt-core              launcher logic: SPT API, bundles, patching, launch (no Qt)
launcher/crates/spt-launcher-qt       cxx-qt bridge + QML UI
scripts/spt-additions                 the installer (based on upstream 1.2.7)
scripts/metadata.conf                 URLs used by the installer
scripts/steam-shortcut.py             Steam non-Steam game entries
```

## Credits

- The `spt-additions` installer and the Linux groundwork are by **MadByte** and the
  [SPT-Linux-Guide contributors](https://github.com/MadByteDE/SPT-Linux-Guide#credits).
- Thanks to the NixOS folks in [PR #14](https://github.com/MadByteDE/SPT-Linux-Guide/pull/14).
- [SPT](https://sp-tarkov.com) is made by the SPT team.

MIT licensed, see [LICENSE](LICENSE). Help & community: [SPT Discord](https://discord.com/invite/Xn9msqQZan).
