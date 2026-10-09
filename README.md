# AION 2 Meter

A passive DPS meter for AION 2 on Windows: an app window with live and past fights, plus an always-on-top overlay
for the game or a second monitor.

It works like the other community meters: it **reads** the game's own network traffic through Npcap and decodes
it. It never sends or changes packets, never touches game memory and never injects into the game. Third-party
tools may still break the game's terms of service, so use it at your own risk.

## For players

1. Install [Npcap](https://npcap.com/#download) and tick **"WinPcap API-compatible Mode"**.
2. Download `AION 2 Meter_x.y.z_x64-setup.exe` from Releases and install it. Windows SmartScreen will warn about
   an unknown publisher because the app isn't code-signed; choose *More info → Run anyway*.
3. Start the meter, then the game. Play in **borderless windowed** mode so the overlay shows.

| Hotkey | Does |
|---|---|
| Ctrl+Shift+L | Lock / unlock the overlay (locked = clicks go through to the game) |
| Ctrl+Shift+O | Show / hide the overlay |
| Ctrl+Shift+R | Start a new encounter |

## For developers

```text
crates/meter-core   Pure Rust: TCP reassembly → frame decoder → packet parser → combat tracker. No UI, no driver.
src-tauri           Tauri v2 shell: live capture (pcap crate), settings, tray, hotkeys, pushes snapshots to the UI.
src/app             Main window (React): Live, History, Settings, Setup.
src/overlay         Overlay window (React): compact bars, drag to move, click-through when locked.
```

Requirements: Rust (stable), Node 22, and on Windows the [Npcap SDK](https://npcap.com/#download) for linking
(set `LIBPCAP_LIBDIR` to its `Lib\x64` folder) plus Npcap itself for running.

```sh
npm install
npm run tauri dev                 # run the app with hot reload
cargo test -p meter-core          # decoder tests, run anywhere
npm run tauri build               # Windows installer in target/release/bundle/
```

No game handy? Turn on **Demo mode** in Settings for made-up fights.

### Working on the decoder offline

Record a fight in Wireshark (filter `tcp.port == 13328`), save it as `.pcapng`, then:

```sh
cargo run -p meter-core --bin replay -- fight.pcapng              # encounters and DPS
cargo run -p meter-core --bin replay -- fight.pcapng --opcodes    # packet counts per opcode
cargo run -p meter-core --bin replay -- fight.pcapng --dump 0438  # every damage packet as hex
cargo run -p meter-core --bin replay -- fight.pcapng --events     # every decoded event
```

Captures are git-ignored; they contain your own traffic, so don't commit or share them casually.
See [docs/PROTOCOL.md](docs/PROTOCOL.md) for the wire format and [docs/REVERSING.md](docs/REVERSING.md)
for how to find new fields or fix things after a game patch.

### Building for friends

Push a tag (`git tag v0.1.0 && git push --tags`). The **Windows build** workflow builds the installer and
attaches it to a draft GitHub release. You can also run the workflow by hand from the Actions tab.
