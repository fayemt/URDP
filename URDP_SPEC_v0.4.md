# URDP Specification & Datasheet (Draft v0.1)

## Overview

URDP (Universal Referenced Datagram Protocol) is a family of transport profiles that minimise on-wire bytes by sending references into a shared codebook ("codex") plus a trickle of parity, allowing receivers to reconstruct the original data locally before any retransmission would complete. It retains TCP ‑‑like reliability on clean links and can degrade gracefully to a UDP+FEC baseline on noisy or unknown data.

The protocol is designed as a drop ‑‑in on top of QUIC but could be mapped to other transports.  It introduces the concept of **domains** (data types), **lanes** (priority/latency classes) and **codices** (shared, signed model packs).

### Why URDP?
- Reduce bandwidth by exploiting structure: logs, telemetry, game state and large downloads can be compressed to their conditional entropy.
- Maintain reliability: CRC and optional HMAC detect and reject wrong reconstructions; parity repair keeps latencies predictable.
- Backwards compatibility: if no codex is available or decoding exceeds CPU budgets, URDP falls back to systematic raw+FEC.

## Terminology

- **Codex**: Signed pack of tables, models, dictionaries or codebooks used by both sender and receiver to compute references and decoding priors. Referenced by a 256 ‑bit hash.
- **Lane**: Per ‑block priority bitmask. Gold = must arrive on time (authoritative input), Silver = important state or code, Bronze = cosmetic data.
- **REF**: The reference payload included at the start of a block. Informs the decoder of the bin or latent index for the block.
- **PARITY**: Rateless parity slices. The sender drips parity until the receiver acknowledges success via CRC.
- **RAW_POLICY**: Whether a block includes systematic raw data (systematic), parity‑only or a micro ‑systematic seed.

## Base Protocol (URDP ‑G)

1. **Codex negotiation**: Sender sends a `CODEX_OFFER` listing available codexes (names, hash, domains, expected bits per byte, decode cost).  Receiver replies with `CODEX_SELECT` choosing one codex per domain, plus CPU budget and lane policies.  Both commit via `CODEX_COMMIT`. Codex IDs are pinned for the session.
2. **Block header**: Each block carries a varint `block_id`, a `codex_slot` (1 byte), lane/flags (including RAW policy), and an 8 ‑byte session tag derived from the codex map.  A CRC32 covers the block and an optional HMAC over the original provides authenticity.
3. **Data flow**: For each block, the sender emits the reference first, then parity slices paced with QUIC’s congestion window.  If the decoder passes CRC early, the sender stops sending parity.  Otherwise parity continues until the baseline deadline (equivalent to raw+FEC).
4. **Reliability modes**: 
   - *Strict*: URDP frames ride inside QUIC streams, giving TCP ‑equivalent reliability.
   - *Low ‑latency*: URDP uses QUIC datagrams; parity repair removes retransmission RTTs.

## Profiles

The core can be specialised by adjusting block size, lanes and codex:

- **URDP ‑X** (Real ‑time state): 16–64 KiB blocks, gold lane always systematic; silver/bronze parity‑only.  Deadlines align with frame or tick budgets.
- **URDP ‑L** (Lossless downloads): 64–256 KiB atoms.  No systematic raw; parity‑only baseline equals UDP+FEC.  A recommended base codex can cut 80 GB games down to a few GB on wire.
- **URDP ‑S** (State sync/replication): Adds Merkle manifests and idempotent writes; uses cross‑object fountain codes.
- **URDP ‑M** (Multicast): Parity slices are coded across flows; receivers collect any K slices.
- **URDP ‑P** (P2P): Peers contribute parity; gold remains authenticated by an authority.
- **URDP ‑I** (IoT): Small codexes, strict CPU budgets, simpler codes.
- **URDP ‑EX** (High compute): Uses long LDPC codes, heavy codexes and GPU/FPGA decode for inter‑DC transfers; still falls back gracefully.

## Lane Policy and RAW policy

| Lane   | Usage                      | Default RAW_POLICY         |
|-------|----------------------------|----------------------------|
| Gold  | Authoritative commands, must not miss deadlines | Systematic or micro‑systematic (5%) |
| Silver| State deltas, important updates | Parity‑only |
| Bronze| Cosmetic or optional data   | Parity‑only (droppable) |

Receivers advertise CPU budgets; if decoding repeatedly exceeds the budget, URDP demotes that domain to micro‑systematic or full systematic, increasing on‑wire bytes to meet latency targets.

## Security and Integrity

- **Model pinning**: Codex hash and publisher signature included in CODEX_OFFER; mismatch aborts.
- **Integrity**: CRC32 per block; HMAC(original) every N bytes (e.g., 1 MiB) prevents plausible but wrong reconstructions.
- **Confidentiality**: URDP frames are compressed before QUIC’s AEAD, maintaining end‑to‑end encryption.
- **DoS resistance**: CPU and parity budgets cap resource use; drop detection triggers fallback to raw.

## Error handling

URDP defines error codes such as `ERR_CODEX_MISMATCH`, `ERR_BAD_SIG`, `ERR_CPU_BUDGET`, `ERR_EPSILON_EXCEEDED`.  On recoverable errors (e.g., CPU budget exceeded), the sender demotes the RAW policy or increases parity; on fatal errors (bad signature), the session aborts.

## Appendices (Normative)

- **Canonical CBOR**: The CODEX negotiation messages use deterministic CBOR encoding.  Fields are sorted by key and maps are encoded as major type 5 with definite length.
- **State machines**: Diagrams show sender/receiver flows for negotiation and per‑block processing.
- **QUIC mapping**: URDP control messages are sent on a dedicated control stream; REF and PARITY frames use DATAGRAM frames.  Session HMAC key is derived from the QUIC exporter.
- **Test vectors**: See Appendix N for byte‑exact examples of varints, headers and signed offers.
- **Operational modes**: Chill, Balanced and Turbo (uses more CPU) modes define CPU caps, decode budgets and default raw policies.  A live governor automatically demotes or promotes modes based on CPU usage and user interaction.

---

**Draft compiled: 30 Aug 2025.**  This document is a high‑level synthesis of the URDP specification and does not replace the canonical appendices and test vectors.
