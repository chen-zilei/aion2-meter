# Finding things in the packets

The approach that works for this game: do something simple and countable in game, capture it, and look for it.

1. **Capture labelled fights.** Wireshark, filter `tcp.port == 13328`, one file per experiment:
   *one skill ×10 on a dummy*, *crit vs non-crit*, *front vs back attack*, *with a summon*, *party of 2*.
2. **Count opcodes.** `replay file.pcapng --opcodes`. Something you did 10 times usually shows up as an opcode
   whose count moved by 10 compared with an idle capture.
3. **Dump bodies.** `replay file.pcapng --dump 0438`. Line them up and look for the fields that change.
   Damage numbers from the in-game Combat Analysis window (Ctrl+X) are your ground truth: encode a number as a
   varint and search for those bytes.
4. **Diff.** Two captures that differ in one thing (crit or not) usually differ in one byte or bit.
5. **Lock it in with a test.** Put a small body you understand into a unit test in `parser.rs` before changing
   the parser, so a later fix can't silently break it.

## After a game patch

If the meter suddenly shows nothing:

- `replay --opcodes` on a fresh capture. If the heartbeat count is zero, framing or the port changed. If it's
  fine but `Damage` is missing, the damage opcode moved: look for a new high-count opcode during combat.
- Check whether the community meters have pushed fixes; their commit history is the fastest changelog.

## Ground rules

Read-only, always. Don't send, replay or modify packets and don't touch game memory: that's what gets
accounts banned, and it isn't needed for a meter.
