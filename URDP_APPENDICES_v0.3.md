# URDP Protocol Appendices v0.3

These appendices accompany the core specification of the Universal Referenced Datagram Protocol (URDP).  They provide normative guidance on user‑impact policies, wire formats, error handling, security hardening and interoperability.  The goal is to make URDP safe and efficient for everyone—from desktop gamers to datacenter operators—without monopolising resources or degrading fairness on shared networks.

## Appendix F — Operational Modes & QoS (User‑Impact Guardrails)

### F.1 Design goal

URDP “mints bandwidth” by spending additional CPU.  For consumers, the protocol must respect user activity, power budgets and thermal limits.  The receiver enforces per‑block compute ceilings and adapts on the fly to avoid turning the machine into a space heater.

### F.2 Modes (receiver policy)

Endpoints negotiate a policy when selecting codexes.  Each mode defines a CPU budget, decode time budget per 64 KiB block, baseline coding policy and codex size limits.  These presets may be adjusted by the sender in response to network conditions (e.g. raising FEC overhead when loss spikes).

| Mode             | Purpose                                    | CPU cap (per core)          | Decode budget (per 64 KiB)            | RAW policy bias                      | FEC ε (overhead) | Codex size cap |
|------------------|---------------------------------------------|-----------------------------|----------------------------------------|--------------------------------------|-----------------|---------------|
| **Chill**        | Minimise impact while the user is active    | ≤10 % desktop; ≤10 % laptop | ≤0.8 ms desktop / ≤1.5 ms laptop       | Prefer **Systematic**; silver uses **Micro(5 %)**; bronze uses **ParityOnly** | 6–10 %         | 128–300 MB    |
| **Balanced**     | Default for most users                      | ≤25 % desktop; ≤25 % laptop | ≤1.0 ms desktop / ≤2.0 ms laptop       | Gold uses **Systematic**; silver and bronze use **ParityOnly**               | 5–8 %          | 300 MB–1 GB   |
| **Turbo (uses more CPU)** | Maximum savings on fast links or idle machines | 50–70 % desktop; 50–70 % laptop | ≤1.5 ms desktop / ≤2.5 ms laptop | Gold uses **Micro(5 %)**; others use **ParityOnly**                         | 3–6 %          | 1–4 GB        |

**RAW policy**: *Systematic* sends the block’s raw symbols alongside parity; *Micro(5 %)* sends only the first 5 % of raw symbols plus parity; *ParityOnly* sends no raw symbols, relying exclusively on parity and the codex.  Gold lanes (latency‑critical data) always have at least micro‑systematic to avoid jitter on clean links.

### F.3 Live governor

The receiver implements a governor to enforce the chosen policy:

- **CPU guard**: if the moving‑average CPU usage exceeds the cap, the receiver demotes the lane’s RAW policy (e.g. from ParityOnly to Micro) and signals the sender via a `NEED` hint.
- **UX guard**: upon user interaction or animation pressure, the receiver temporarily lowers the mode to Chill for 2–5 seconds.
- **Thermal/battery guard**: on laptops or mobile, a hot or low‑battery state reduces the decode budget by ~30 % and increases FEC ε by 2–3 percentage points.
- **Foreground awareness**: URDP threads run at background priority and with low I/O priority (via cgroups, job objects or `nice`).

### F.4 Break‑even rule

The receiver uses a simple heuristic to decide whether to attempt codex decoding or fall back to raw delivery:

> **Decode if:** (bytes saved / link Bps) ≥ (decode time + margin)

The margin is ~0.5 ms on desktops and ~1.0 ms on laptops.  If the expected on‑wire savings do not cover the decode budget, the receiver treats the block as incompressible and uses systematic or micro‑systematic delivery.

### F.5 Scheduling & OS hooks

URDP uses QUIC pacing to align parity transmissions with congestion control.  Decoders run on background threads and may offload heavy belief‑propagation to GPUs when idle.  Operating systems should assign URDP processes a lower priority class so other applications remain responsive.

### F.6 UX & telemetry

User interfaces should display ETA and a “CPU impact” badge (Low/Medium/High).  Telemetry counters include MB saved, CPU percentage, FEC ε used and the early‑CRC hit rate.  Users may switch modes at any time.

### F.7 Defaults for home machines

The protocol starts in **Balanced** mode.  It automatically demotes to **Chill** upon sustained input or two CPU‑cap breaches within 30 seconds.  It upgrades to **Turbo (uses more CPU)** only when the machine is idle and the link is ≥100 Mbps.  These defaults ensure that everyday downloads do not interfere with day‑to‑day computing tasks.

## Appendix G — Wire Schemas: CBOR and Protobuf

URDP control messages (`CODEX_OFFER`, `CODEX_SELECT` and `CODEX_COMMIT`) are encoded as deterministic CBOR and signed by the publisher.  For compatibility with different stacks, equivalent protobuf definitions are provided.  Key points:

- **CODEX_OFFER**: contains an `offer_id`, and a list of codex descriptors.  Each descriptor includes the codex’s BLAKE3‑256 hash (`codex_id`), name, semantic version, vendor identifier, supported domains (e.g. `texture/BC7`, `text/utf8`), expected bits‑per‑byte and decode time hints, pack size, a signature by the publisher and an optional licence URL.
- **CODEX_SELECT**: identifies the chosen codex per domain (`{domain: codex_id}`) and conveys the receiver’s policy (e.g. Balanced).  It may include a digest of installed codex packs to avoid unnecessary offers.
- **CODEX_COMMIT**: echoes the final domain–codex mapping and includes a `session_codex_map_id`—a BLAKE3 hash of the canonicalised mapping.  Fallback flags indicate domains that will auto‑degrade to raw if decode fails.

**Canonicalisation rules**: maps must be encoded with lexicographically sorted keys, integers use the shortest representation, and strings are encoded as UTF‑8.  Always produce deterministic encodings (e.g. using `BTreeMap` in Rust).  Signatures cover the canonical CBOR payload; verify them using Ed25519.

**Session tag**: to avoid leaking full hashes on every block, data frames include only a 1‑byte `codex_slot` and an 8‑byte `session_tag`.  The session tag is computed as `HMAC_SHA256(session_codex_map_id, exporter_secret)` truncated to 8 bytes.

## Appendix H — User Interface Strings

Front‑end implementations should provide clear descriptions for modes and actions.  Suggested strings:

- **Mode selector**: “Performance mode: Chill / Balanced / Turbo (uses more CPU)”
- **Chill tooltip**: “Minimise CPU use and impact on other tasks.”
- **Balanced tooltip**: “Default mode—saves bandwidth without slowing your computer.”
- **Turbo (uses more CPU) tooltip**: “Use more CPU to save as much bandwidth as possible.”
- **CPU impact badge**: “Low”, “Medium” or “High” depending on recent decode budgets.
- **Codex install prompt**: “A recommended codex pack is available (v2.0, 2 GB).  Install now to reduce download size by ~14 GB?”

## Appendix I — Error Codes & Recovery

URDP uses a small set of 16‑bit error codes to signal exceptional conditions.  Upon receiving an error, the peer must take the mandated action:

| Code                   | Description                                                         | Mandatory receiver action                             |
|------------------------|---------------------------------------------------------------------|-------------------------------------------------------|
| **0x0001 ERR_CODEX_MISMATCH** | Session codex map does not match committed map.               | Abort the session; fall back to raw transfer.        |
| **0x0002 ERR_BAD_SIG**        | Signature verification failed on an OFFER or COMMIT.           | Reject the offer/commit; request a new offer.        |
| **0x0003 ERR_CPU_BUDGET**     | Decoding exceeded the per‑block CPU budget.                   | Demote RAW policy (e.g. ParityOnly→Micro→Systematic).|
| **0x0004 ERR_EPSILON_EXCEEDED** | Parity overhead exceeded the maximum allowed for this domain. | Switch to raw delivery for this domain.              |
| **0x0005 ERR_UNSUPPORTED_DOMAIN** | The offered codex does not support a domain in the stream. | Skip compression for that domain; use raw+FEC.       |
| **0x0006 ERR_DECODE_FAIL**    | Decode did not succeed after maximum iterations or parity.     | Treat the block as corrupt; request retransmission.  |

The sender must honour `NEED` hints indicating CPU or epsilon limits have been hit.  Repeated errors should trigger a switch to systematic delivery until the end of the session or until renegotiation.

## Appendix J — Deterministic CBOR & Signing

To prevent ambiguous encodings and signature malleability, all URDP control messages use deterministic CBOR:

1. **Sorted keys**: maps are encoded with keys in lexicographic order.
2. **Shortest integer encoding**: positive integers use the minimal CBOR representation.
3. **No indefinite items**: arrays and maps must specify their lengths up front.
4. **No floating points**: codex descriptors never use floats; hints are integers or strings.

Before signing, serialise the message to deterministic CBOR.  The preimage for the signature is exactly this canonical representation.  Use Ed25519 for signing.  Verify signatures with constant‑time comparison.

`session_codex_map_id` is computed as `BLAKE3(canonical_map)` truncated to 16 bytes.  The 8‑byte `session_tag` included in data frames derives from `HMAC_SHA256(session_codex_map_id, exporter_secret)` truncated to 8 bytes.  Always compare tags in constant time.

## Appendix K — State Machines

The protocol defines explicit sender and receiver state machines.  In pseudocode:

**Sender**:

1. For each block, assign a block identifier and determine its lane (Gold/Silver/Bronze).
2. Emit a REF frame containing the block’s codex slot, lane flags and session tag.
3. Start sending parity slices at a slope determined by the expected entropy and FEC ε.
4. On receiving `ACK_PASS`, stop parity and move to the next block.
5. If the parity budget expires or a `NEED` is received indicating CPU exhaustion, increase FEC overhead or demote the RAW policy.

**Receiver**:

1. Upon a new REF, start decoding the block using the codex priors.  Apply belief‑propagation or beam search while enforcing the per‑block CPU budget.
2. When the CRC passes, send `ACK_PASS(block_id)` and discard any further parity for that block.
3. If the decode budget expires or parity consumption exceeds ε, demote the RAW policy (ParityOnly→Micro→Systematic) and send a `NEED` hint to the sender.
4. If decode still fails (ERR_DECODE_FAIL), treat the block as corrupt and request retransmission via reliable channels.

These state machines ensure deterministic behaviour and clear fallbacks when decoding or resources become constrained.

## Appendix L — QUIC Mapping

URDP leverages QUIC as its transport substrate:

- **Data frames**: REF and PARITY frames travel over QUIC DATAGRAMs.  Each datagram carries at most one URDP frame.
- **Control frames**: CODEX offers, selects, commits, ACKs and NEEDs use QUIC streams (reliable, in‑order).
- **Security binding**: derive an exporter secret from the QUIC TLS handshake (e.g. via `EXPORTER-URDP-KEY`) and use it to compute the session tag.  Bind the session to the connection’s ID to prevent replay or downgrade.
- **Fairness**: parity and raw bytes obey QUIC’s congestion controller (e.g. BBR or CUBIC).  URDP never sends more data than a competing TCP/QUIC flow would.
- **Path migration**: because data frames carry only a short session tag, migration to a new path does not alter the session.  QUIC handles encryption and loss recovery.

## Appendix M — Security Hardening

URDP incorporates multiple mechanisms to defend against tampering and abuse:

- **Anti‑downgrade**: the codex descriptor and the mapping are signed by the publisher and pinned via `session_codex_map_id`.  A man‑in‑the‑middle cannot silently strip a newer codex from an offer without causing a signature failure.
- **Replay binding**: the session tag binds the codex map to the specific QUIC connection.  Frames from a different session are rejected.
- **DoS mitigation**: per‑block CPU budgets and FEC ε limits prevent an adversary from forcing excessive decode work.  The receiver can demote to raw delivery at any time.
- **Privacy mode**: receivers may elect to reveal only the chosen codex IDs rather than the full set of installed packs.  For additional privacy, a private set intersection mechanism can be employed at the cost of extra latency.
- **Sandboxing**: codex packs contain data (tables, weights, dictionaries), not executable code.  Decoders should run untrusted code in sandboxes when dynamic kernels are permitted.  Heavy decode paths should be isolated from the UI thread.

## Appendix N — Test Vectors

To facilitate cross‑implementation interoperability, URDP provides canonical test vectors:

- **Varint examples**: the block identifier `1` encodes as `0x01`; `300` encodes as `0xAC 0x02` (little‑endian base‑128).  Use these to verify varint decoders.
- **REF header**: a sample header with `block_id=5`, `codex_slot=2`, `lane=Silver`, flags set for `ParityOnly`, and an 8‑byte session tag of `0x01 0x02 0x03 0x04 0x05 0x06 0x07 0x08` yields the byte sequence:

  `05 02 40 00 01 02 03 04 05 06 07 08`

  The third byte (`0x40`) encodes the lane and flags; here `0x40` means lane=1 (Silver) and `RAW_POLICY=ParityOnly`.

- **CODEX_OFFER preimage**: the canonical CBOR encoding of an offer containing one codex descriptor has the hex representation:

  `A2 68 6F 66 66 65 72 5F 69 64 01 6D 63 6F 64 65 78 5F 6C 69 73 74 81 A7 67 63 6F 64 65 78 5F 69 64 50 <codex‑id> 64 6E 61 6D 65 6A 55 52 44 50 20 76 32 2E 30 69 76 65 6E 64 6F 72 5F 69 64 66 77 69 64 67 65 74 64 6F 6D 61 69 6E 73 82 6A 74 65 78 74 75 72 65 2F 42 43 37 66 74 65 78 74 2F 75 74 66 38 69 68 69 6E 74 73 82 66 65 78 70 5F 62 70 62 01 69 65 78 70 5F 64 65 63 5F 75 73 05 6C 70 61 63 6B 5F 73 69 7A 65 19 01 F4 63 73 69 67 58 40 <signature>`

  (where `<codex‑id>` is the 32‑byte BLAKE3 hash and `<signature>` is the 64‑byte Ed25519 signature).  Use this vector to test canonicalisation and signing.

- **ACK/NEED frames**: an `ACK_PASS` for `block_id=5` encodes as `05`.  A `NEED` indicating CPU exhaustion uses a prefix byte `0x80` followed by the block identifier (here `0x80 05`).

Developers should use these vectors to validate their encoders and decoders.  Implementations must match the exact byte sequences; any deviation indicates a canonicalisation bug.

## Appendix O — Loss & Drop Handling

URDP frames travel over QUIC datagrams; if a REF or PARITY datagram is lost, the receiver simply waits for additional parity slices rather than issuing a retransmission.  The protocol uses adaptive FEC overhead ε based on observed loss; receivers may send `NEED` hints to request more parity.  If a REF is lost entirely, parity ensures that the block still completes on time; you only lose the opportunity for early decode.  Burst loss resilience is achieved by interleaving parity across multiple blocks within a sliding window.  Because all URDP data is paced by QUIC’s congestion control, drop recovery never transmits more than a conventional TCP/QUIC flow would.
