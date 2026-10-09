# AION 2 wire format (as far as we know)

Community findings for the post-June-2026 client, which Global launched on. Main reference:
[cyberbadger6969/aion2-dps-meter](https://github.com/cyberbadger6969/aion2-dps-meter). None of it is official and
patches can change it. The code in `crates/meter-core` is our own implementation of these findings.

## Transport

- TCP, server port **13328** (configurable in Settings). Only server→client traffic is decoded.
- The address the client connects to is a Cloudflare relay near the player, which acknowledges TCP segments
  itself, so TCP timing only measures the hop to the relay.
- Client→server frames use the same length prefix, but their opcode and body are encrypted. The client sends an
  11-byte heartbeat frame about every 50 ms and a 13-byte ping frame every 10 s.
- **Not encrypted.** Some packets are LZ4-compressed.
- With a ping reducer or VPN, the traffic shows up on the loopback adapter, which the app also captures.

## Framing (`frame.rs`)

```text
frame   = varint len, [one byte in 0xF0..=0xFE], payload        frame size on the wire = len + varint bytes − 4
payload = opcode (2 bytes, big-endian as written below) body
        | FF FF, u32 raw_size, LZ4 block                         the block decompresses to more frames
```

- Integers inside bodies are little-endian; most are LEB128 varints.
- `0x00` bytes between frames are padding.
- Heartbeat frame `0E 00 36` + 8 bytes, about 20 per second: u64 LE server Unix ms, stepping by 50. Used to
  re-align after packet loss and for the ping readout.

## Opcodes (`opcodes.rs`)

| Opcode | Meaning | Decoded |
|---|---|---|
| `00 36` | Heartbeat (server Unix ms) | yes |
| `03 36` | Pong for the client's 13-byte ping: `00 00`, u64 echo of the client's clock, u64 server Unix ms on receipt | ping |
| `33 36` | Own character (id, name, server, class, level) | name |
| `45 36` | Another player | name |
| `41 36` | Spawn: NPC, summon, effect entity, with owner link | NPC code, max HP |
| `42 36` | Death | yes |
| `21 36` | Map load | no |
| `02 38` | Cast | no |
| `04 38` | Damage | yes |
| `05 38` | DoT / HoT tick | yes |
| `00 8D` | Remaining HP | no |
| `02 97` | Party roster | no |

## Damage `04 38` (`parser.rs`)

```text
target varint
switch varint        low 4 bits = layout (4..7; 0 = cast marker with no damage), 0x20 = extra hit list follows
flag varint
actor varint
skill u32
hit uid u8
dmg type varint      3 = critical
block                8 bytes (layout 4) or 11 bytes (5..7): [mods, 00, dir, ...]
                     mods 0x02 parry, 0x04 perfect, 0x08 double, 0x20 heavy; dir 1 = back, 2 = front
[0 pad] scalar varint, damage varint
tail                 layout 4: one small varint; 0x20: count + that many extra-hit varints
```

More records may follow, each prefixed with `01 00`.

## DoT `05 38`

```text
target varint, effect u8 (02/0A damage, 01/09/0B heal), actor varint, varint, skill u32 (×100), amount varint
```

## Identity `33 36` / `45 36`

```text
id varint, u32 mask, u8 flags (bit 0 = has name), u8 len, utf8 name, then server u16, class u32, ...
```

The game only sends these when a player comes into view or on a loading screen, but the same records also ride
inside other packets, mid-body and inside LZ4 bundles embedded in a larger packet (the own record repeats that way
every few minutes). The parser scans unknown packets for both, which is why names must validate strictly: up to 16
letters or digits with at least one letter. Tutorial names (`$` + random characters) are ignored.

## Spawn `41 36`

```text
id varint            ids above 1,000,000 fold into the combat id space: (raw & 0x3FFF) | 0x4000
mask u32             first byte = kind; 5F 1C 1F 1D 5D are summons, spirits, pets and skill effects
...                  variable-length fields
npc_code u24         right before the marker, searched for within 60 bytes of the mask
00 (00|40) 02        marker
x y z f32
...  01 cur_hp varint max_hp varint
```

## Known gaps

- Summons and ground effects deal damage under their own entity id. Folding them into the owner needs the owner
  link in the spawn packet (`41 36`), which isn't decoded yet.
- A player can have several entity ids (a stable one, a combat one, a new one after a dungeon re-bind).
- Skill names come from community tables extracted from the game client (see `crates/meter-core/src/names.rs`);
  the packets themselves only carry codes.
- Names are not kept across meter restarts, so after restarting mid-zone a player stays `#id` until the game
  re-sends their record.
