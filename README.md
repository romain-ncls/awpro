<div align="center">

# 🎧 awpro

**A command-line controller for the Dell Alienware Pro Wireless Gaming Headset.**

No Alienware Command Center, no Windows. Just one static binary.

[![Release](https://img.shields.io/github/v/release/romain-ncls/awpro?style=flat-square)](https://github.com/romain-ncls/awpro/releases/latest)
[![Build](https://img.shields.io/github/actions/workflow/status/romain-ncls/awpro/release.yaml?style=flat-square&label=release)](https://github.com/romain-ncls/awpro/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
![Platform: Linux](https://img.shields.io/badge/platform-linux-lightgrey?style=flat-square)

</div>

```console
$ awpro status
battery           82%
anc               on
mic               unmuted
mic noise-cancel  on
sidetone          off
power saving      off
power auto-off    30 minutes
wireless link     up
```

## ✨ Features

- 🔋 **Battery**: read the charge level and charging state
- 🔇 **ANC**: switch between off, noise cancelling, and transparency (levels 1–5)
- 🎙️ **Microphone**: mute, unmute, and toggle mic noise cancellation
- 👂 **Sidetone**: set how much of your own voice you hear (off, 1–5)
- ⏻ **Power**: configure the auto power-off timer and battery-saving mode
- 👀 **Watch**: stream the changes the headset pushes, including ones made with its own buttons
- 🤖 **`--json`** on every command, for scripts and status bars
- 🔌 Works over the **2.4 GHz dongle** or the **USB-C cable**, picked automatically

## 📦 Installation

### Debian / Ubuntu

Download the `.deb` from the [latest release](https://github.com/romain-ncls/awpro/releases/latest) and install it:

```sh
sudo apt install ./awpro_*_amd64.deb
```

The package ships the udev rule and reloads udev for you, so an already-plugged-in headset works right away.

### NixOS (flake)

```nix
{
  inputs.awpro.url = "github:romain-ncls/awpro";

  outputs = { nixpkgs, awpro, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      modules = [
        awpro.nixosModules.default
        { programs.awpro.enable = true; }
      ];
    };
  };
}
```

The module installs the binary and the udev rule. To try it without installing:

```sh
nix run github:romain-ncls/awpro -- status
```

### Any Linux (static binary)

Each [release](https://github.com/romain-ncls/awpro/releases/latest) includes a fully static musl binary with no runtime dependencies:

```sh
tar -xzf awpro-*-x86_64-linux.tar.gz
sudo install -m755 awpro /usr/local/bin/
sudo install -m644 packaging/70-awpro.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw --action=change
```

Verify the download against `SHA256SUMS`, which is attached to the release.

### From source

```sh
cargo install --git https://github.com/romain-ncls/awpro
```

You still need the udev rule from [`packaging/70-awpro.rules`](packaging/70-awpro.rules) to use the tool without `sudo`.

> [!NOTE]
> **Why a udev rule?** Linux only lets root open raw HID devices by default. The rule tags the dongle (`413c:a529`) and the headset's USB-C interface (`413c:a528`) with `uaccess`, which gives the logged-in user access to them.

## 🚀 Usage

### Reading state

```sh
awpro status              # every readable setting at once
awpro battery             # 82%
awpro info                # firmware version, product id, transport, link
awpro get anc             # on | off | transparency (level N)
awpro get mic             # muted | unmuted
awpro get mic noise-cancel
awpro get sidetone
awpro get power saving
awpro get power auto-off
```

### Changing settings

```sh
awpro anc on
awpro anc off
awpro anc transparency --level 4        # 1–5, default 3

awpro mic mute
awpro mic unmute
awpro mic noise-cancel on               # or off

awpro sidetone 2                        # off, 1–5

awpro power auto-off 60                 # off, 15, 30, 45, 60 (minutes)
awpro power saving on --threshold 20    # enable below 20% battery
awpro power saving off
```

Setters print nothing when they succeed. If the headset never acknowledges a write, for example because it is off or out of range, you get a warning on stderr.

### Watching for changes

```sh
awpro watch
```

This streams one line per change the headset pushes, until you interrupt it. It picks up presses of the headset's physical buttons, which the query commands cannot see. Sidetone, power saving and auto power-off are never pushed, so they don't show up here.

It stops cleanly when piped into `head`, `grep -q` and similar tools:

```sh
awpro watch | grep -m1 -w muted && notify-send "Mic muted"
```

### JSON output

Pass `--json` to any command to get one JSON object per line:

```console
$ awpro --json status
{"anc":{"mode":"on"},"battery":{"charging":false,"level":82},"mic":{"muted":false,"noise_cancel":true},"power":{"auto_off":{"enabled":true,"minutes":30},"saving":{"enabled":false}},"sidetone":0,"wireless_link":{"up":true}}
```

With `--json`, setters print the new state along with `"ok"` and `"acknowledged"`. Errors also print a `{"error": {"kind": …, "message": …}}` object on stdout, so a consumer parsing stdout never gets empty output.

This makes `awpro` easy to wire into a status bar. Here's a [Waybar](https://github.com/Alexays/Waybar) module, for example:

```jsonc
"custom/headset": {
  "exec": "awpro --json battery | jq -r '\"🎧 \\(.level)%\"'",
  "interval": 60
}
```

## 🔌 Dongle or cable?

`awpro` looks for the USB-C connection first and falls back to the dongle. Plugging the headset in over USB-C drops its 2.4 GHz link, so whichever one is present is the right one, and there's nothing to configure.

When the dongle is plugged in but the headset is off or out of range, `awpro` tells you the wireless link is down instead of reporting a bare timeout.

## 🛠️ Development

```sh
nix develop        # or bring your own Rust toolchain
cargo test
cargo build --release
```

Releases are cut by [release-plz](https://release-plz.dev) from [Conventional Commits](https://www.conventionalcommits.org). CI then attaches the static binary, the `.deb` and the checksums to the GitHub release. See [CHANGELOG.md](CHANGELOG.md) for the history.

## ⚠️ Disclaimer

This is an unofficial tool. It is not affiliated with or endorsed by Dell or Alienware. The protocol was reverse-engineered, so use it at your own risk.

## 📄 License

[MIT](LICENSE) © Romain Nicolas
