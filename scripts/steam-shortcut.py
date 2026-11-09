#!/usr/bin/env python3
"""Add / remove non-Steam game shortcuts (Steam's binary shortcuts.vdf).

Part of the SPT NixOS port of spt-additions. Standard library only.

  steam-shortcut.py add    --name N --exe PATH [--start-dir D] [--icon I]
                           [--launch-options S] [--cover IMG] [--tag T] [--user auto|all|ID]
  steam-shortcut.py remove --name N [--user auto|all|ID]
  steam-shortcut.py users  [--user auto|all|ID]

Steam must NOT be running: it rewrites shortcuts.vdf on exit.
"""

import argparse
import os
import re
import shutil
import struct
import sys
import zlib

# Binary VDF value types
T_MAP, T_STR, T_INT, T_FLOAT, T_PTR, T_WSTR, T_COLOR, T_U64, T_END, T_I64 = (
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10)
STEAMID64_BASE = 76561197960265728


class VdfError(Exception):
    pass


# ----------------------------------------------------------------------------
# Binary VDF (de)serialisation - keeps value types so unknown fields round-trip
# ----------------------------------------------------------------------------

def _read_cstr(data, i):
    end = data.index(b"\x00", i)
    return data[i:end].decode("utf-8", "surrogateescape"), end + 1


def _parse_map(data, i):
    items = []  # list of (key, type, value) to preserve order & types
    while True:
        if i >= len(data):
            raise VdfError("unexpected end of file")
        t = data[i]
        i += 1
        if t == T_END:
            return items, i
        key, i = _read_cstr(data, i)
        if t == T_MAP:
            val, i = _parse_map(data, i)
        elif t == T_STR:
            val, i = _read_cstr(data, i)
        elif t in (T_INT, T_PTR, T_COLOR):
            val = struct.unpack_from("<i", data, i)[0]
            i += 4
        elif t == T_FLOAT:
            val = struct.unpack_from("<f", data, i)[0]
            i += 4
        elif t == T_U64:
            val = struct.unpack_from("<Q", data, i)[0]
            i += 8
        elif t == T_I64:
            val = struct.unpack_from("<q", data, i)[0]
            i += 8
        else:
            raise VdfError(f"unsupported value type 0x{t:02x} at offset {i - 1}")
        items.append((key, t, val))


def vdf_load(data):
    items, i = _parse_map(data, 0)
    return items


def _dump_map(items, out):
    for key, t, val in items:
        out.append(bytes([t]))
        out.append(key.encode("utf-8", "surrogateescape") + b"\x00")
        if t == T_MAP:
            _dump_map(val, out)
        elif t == T_STR:
            out.append(val.encode("utf-8", "surrogateescape") + b"\x00")
        elif t in (T_INT, T_PTR, T_COLOR):
            out.append(struct.pack("<i", val))
        elif t == T_FLOAT:
            out.append(struct.pack("<f", val))
        elif t == T_U64:
            out.append(struct.pack("<Q", val))
        elif t == T_I64:
            out.append(struct.pack("<q", val))
    out.append(bytes([T_END]))


def vdf_dump(items):
    out = []
    _dump_map(items, out)
    return b"".join(out)


def _get(items, key, default=None):
    for k, _t, v in items:
        if k.lower() == key.lower():
            return v
    return default


# ----------------------------------------------------------------------------
# Steam helpers
# ----------------------------------------------------------------------------

def steam_users(steam_dir, selector):
    userdata = os.path.join(steam_dir, "userdata")
    if not os.path.isdir(userdata):
        raise SystemExit(f"Steam userdata directory not found: {userdata}")
    ids = sorted(d for d in os.listdir(userdata) if d.isdigit() and d != "0"
                 and os.path.isdir(os.path.join(userdata, d)))
    if not ids:
        raise SystemExit("No Steam users found (log into Steam at least once)")

    if selector == "all":
        return ids
    if selector not in ("auto", ""):
        if selector not in ids:
            raise SystemExit(f"Steam user {selector} not found (available: {', '.join(ids)})")
        return [selector]

    # auto: MostRecent=1, else highest timestamp in loginusers.vdf
    best, best_score = None, (-1, -1)
    try:
        with open(os.path.join(steam_dir, "config", "loginusers.vdf"), encoding="utf-8",
                  errors="replace") as f:
            text = f.read()
        for m in re.finditer(r'"(\d{17})"\s*\{(.*?)\}', text, re.S):
            uid = str(int(m.group(1)) - STEAMID64_BASE)
            body = m.group(2)
            recent = re.search(r'"mostrecent"\s*"(\d+)"', body, re.I)
            stamp = re.search(r'"timestamp"\s*"(\d+)"', body, re.I)
            score = (int(recent.group(1)) if recent else 0, int(stamp.group(1)) if stamp else 0)
            if uid in ids and score > best_score:
                best, best_score = uid, score
    except OSError:
        pass
    if best:
        return [best]
    if len(ids) == 1:
        return ids
    raise SystemExit("Multiple Steam users found and none marked as most recent; "
                     f"choose one with --user ({', '.join(ids)}) or --user all")


def shortcut_appid(exe, name):
    """Unsigned 32-bit id Steam uses for non-Steam shortcuts & their grid images."""
    return zlib.crc32((exe + name).encode("utf-8")) | 0x80000000


def to_signed(v):
    return v - (1 << 32) if v >= (1 << 31) else v


def load_shortcuts(path):
    if not os.path.isfile(path) or os.path.getsize(path) == 0:
        return [("shortcuts", T_MAP, [])]
    with open(path, "rb") as f:
        return vdf_load(f.read())


def save_shortcuts(path, items):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.isfile(path):
        shutil.copy2(path, path + ".spt-backup")
    tmp = path + ".spt-tmp"
    with open(tmp, "wb") as f:
        f.write(vdf_dump(items))
    os.replace(tmp, path)


def shortcuts_list(items):
    root = _get(items, "shortcuts")
    if root is None:
        root = []
        items.append(("shortcuts", T_MAP, root))
    return root


def renumber(entries):
    return [(str(i), T_MAP, v) for i, (_k, _t, v) in enumerate(entries)]


def make_entry(appid, name, exe, start_dir, icon, launch_options, tags, existing=None):
    fields = [
        ("appid", T_INT, to_signed(appid)),
        ("AppName", T_STR, name),
        ("Exe", T_STR, exe),
        ("StartDir", T_STR, start_dir),
        ("icon", T_STR, icon),
        ("ShortcutPath", T_STR, ""),
        ("LaunchOptions", T_STR, launch_options),
        ("IsHidden", T_INT, 0),
        ("AllowDesktopConfig", T_INT, 1),
        ("AllowOverlay", T_INT, 1),
        ("OpenVR", T_INT, 0),
        ("Devkit", T_INT, 0),
        ("DevkitGameID", T_STR, ""),
        ("DevkitOverrideAppID", T_INT, 0),
        ("LastPlayTime", T_INT, 0),
        ("FlatpakAppID", T_STR, ""),
        ("tags", T_MAP, [(str(i), T_STR, t) for i, t in enumerate(tags)]),
    ]
    if existing:
        # Keep user-modified values Steam stores (play time, hidden state, ...)
        keep = {"LastPlayTime", "IsHidden", "AllowOverlay", "AllowDesktopConfig", "OpenVR"}
        fields = [(k, t, _get(existing, k, v) if k in keep else v) for k, t, v in fields]
    return fields


def find_entries(entries, name):
    return [i for i, (_k, _t, v) in enumerate(entries)
            if str(_get(v, "AppName", _get(v, "appname", ""))).lower() == name.lower()]


# ----------------------------------------------------------------------------
# Commands
# ----------------------------------------------------------------------------

def cmd_add(args):
    exe = f'"{args.exe}"'
    start_dir = f'"{args.start_dir or os.path.dirname(args.exe)}"'
    appid = shortcut_appid(exe, args.name)
    for uid in steam_users(args.steam_dir, args.user):
        cfg = os.path.join(args.steam_dir, "userdata", uid, "config")
        path = os.path.join(cfg, "shortcuts.vdf")
        items = load_shortcuts(path)
        entries = list(shortcuts_list(items))
        matches = find_entries(entries, args.name)
        existing = entries[matches[0]][2] if matches else None
        entry = make_entry(appid, args.name, exe, start_dir, args.icon or "",
                           args.launch_options or "", args.tag or [], existing)
        if matches:
            entries[matches[0]] = (entries[matches[0]][0], T_MAP, entry)
            for i in reversed(matches[1:]):  # drop duplicates
                del entries[i]
            action = "Updated"
        else:
            entries.append(("", T_MAP, entry))
            action = "Added"
        items = [(k, t, renumber(entries) if k.lower() == "shortcuts" else v)
                 for k, t, v in items]
        save_shortcuts(path, items)

        # Artwork (grid images are keyed by the unsigned shortcut id)
        if args.cover and os.path.isfile(args.cover):
            grid = os.path.join(cfg, "grid")
            os.makedirs(grid, exist_ok=True)
            ext = os.path.splitext(args.cover)[1] or ".png"
            for old in os.listdir(grid):
                if old.startswith(f"{appid}p."):
                    os.remove(os.path.join(grid, old))
            shutil.copyfile(args.cover, os.path.join(grid, f"{appid}p{ext}"))
        print(f"{action} \"{args.name}\" for Steam user {uid} (appid {appid})")


def cmd_remove(args):
    for uid in steam_users(args.steam_dir, args.user):
        cfg = os.path.join(args.steam_dir, "userdata", uid, "config")
        path = os.path.join(cfg, "shortcuts.vdf")
        if not os.path.isfile(path):
            continue
        items = load_shortcuts(path)
        entries = list(shortcuts_list(items))
        matches = find_entries(entries, args.name)
        if not matches:
            print(f"\"{args.name}\" not found for Steam user {uid}")
            continue
        grid = os.path.join(cfg, "grid")
        for i in reversed(matches):
            appid = _get(entries[i][2], "appid")
            if appid is not None and os.path.isdir(grid):
                uappid = appid & 0xFFFFFFFF
                for f in os.listdir(grid):
                    if re.match(rf"^{uappid}(p|_hero|_logo)?\.", f):
                        os.remove(os.path.join(grid, f))
            del entries[i]
        items = [(k, t, renumber(entries) if k.lower() == "shortcuts" else v)
                 for k, t, v in items]
        save_shortcuts(path, items)
        print(f"Removed \"{args.name}\" for Steam user {uid}")


def cmd_users(args):
    print("\n".join(steam_users(args.steam_dir, args.user)))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--steam-dir", default=os.path.expanduser("~/.steam/steam"))
    p.add_argument("--user", default=os.environ.get("SPT_STEAM_USER", "auto"),
                   help="auto (most recent login), all, or a Steam account id")
    sub = p.add_subparsers(dest="cmd", required=True)

    a = sub.add_parser("add")
    a.add_argument("--name", required=True)
    a.add_argument("--exe", required=True)
    a.add_argument("--start-dir")
    a.add_argument("--icon")
    a.add_argument("--launch-options")
    a.add_argument("--cover")
    a.add_argument("--tag", action="append")
    a.set_defaults(func=cmd_add)

    r = sub.add_parser("remove")
    r.add_argument("--name", required=True)
    r.set_defaults(func=cmd_remove)

    u = sub.add_parser("users")
    u.set_defaults(func=cmd_users)

    args = p.parse_args()
    try:
        args.func(args)
    except VdfError as e:
        raise SystemExit(f"Failed to parse shortcuts.vdf: {e} (file left untouched)")


if __name__ == "__main__":
    main()
