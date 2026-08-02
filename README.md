# fluqsr

Send any file to any device on your local network. Peer-to-peer, end-to-end
encrypted, no account and no cloud. The only limits are your disk and your
link speed.

Built with Tauri 2, Rust, and React. Windows, Linux, and macOS.

## Status

Early. The transport, discovery, pairing, and transfer engine work end to end;
the UI is minimal. See [Not done yet](#not-done-yet).

## How it works

**Discovery.** Devices announce themselves over a UDP multicast beacon every
few seconds, and separately over mDNS. Both feed one peer list, because either
can be blocked depending on the network. When neither works, you can type an
address directly.

**Transport.** TCP with TLS 1.3, streamed in 512 KB chunks. On a LAN, kernel
TCP saturates the link and AES-NI makes the encryption free, so the bottleneck
stays where it should be — the disk and the radio.

**Identity.** Each device generates one long-lived Ed25519 keypair on first
run. Its identity is the SHA-256 of that key (an SPKI pin). Certificates are
self-signed and carry no authority; they exist because TLS needs them.

**Pairing.** There is no certificate authority, so the first connection between
two devices shows the same six-digit code on both screens and asks the user to
compare them. The code is derived from *both* devices' fingerprints, so a
man-in-the-middle — who necessarily holds a different key toward each side —
cannot make the two screens agree. Confirming pins the key; every later
transfer is silent.

**Receiving.** Incoming transfers are consent-gated by default. Every path is
validated before anything is written: no traversal, no absolute paths, no
drive letters, no NTFS alternate data streams, no Windows device names.
Everything lands under one folder, existing files are never overwritten, and
received files never get the executable bit. Each file is verified with BLAKE3
on arrival, and interrupted transfers resume from a `.part` file.

## Running it

```bash
npm install
```

```bash
npm run tauri dev
```

Run it on two machines on the same network and they should find each other.

### Tests

```bash
cd src-tauri && cargo test
```

The unit tests cover the pieces; `tests/end_to_end.rs` stands up two real
nodes and transfers between them over TLS on loopback, including the
traversal, impersonation, and overwrite cases.

## Known limitations

- **Client isolation.** Many public, hotel, and campus networks block
  device-to-device traffic at layer 2. No application can work around this;
  use a phone hotspot or a cable.
- **Multi-homed machines.** The multicast beacon joins on the default
  interface, so a machine with several networks may not be discovered on all
  of them. Manual addressing still works.
- **Firewalls.** Windows prompts on first run. Discovery uses UDP 47653,
  transfers TCP 47654.

## Not done yet

- Free-space check before accepting an offer
- Cancel propagation to the peer mid-transfer (currently the connection just
  drops, which the other side reports as a failure)
- A settings screen for auto-accept and unpairing — the backend commands exist
  but nothing calls them yet
- Parallel connections. One stream saturates a LAN link, so this only matters
  if measurement shows small-file batches are round-trip bound.
- IPv6 transfers (discovery prefers IPv4; the listener binds v4)
- Replacing the pairing-code comparison with SPAKE2, so the code
  cryptographically authenticates the channel rather than relying on the user
  actually looking

## History

This repository was previously CrossNotes, a note-taking app. That code is
preserved on the `legacy/crossnotes` branch and tagged `v0.1-crossnotes`.
