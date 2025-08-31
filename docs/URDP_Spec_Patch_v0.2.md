# URDP Spec Patch v0.2 — Wire Contract Lock‑in (Appendix N & P)

This patch extends the URDP specification with normative test vectors, a micro‑systematic example, extended ACK/NEED
frames and a precise binary layout for the new video TLVs used by URDP‑X streams.

## Extended varint test vectors

Varints are encoded using little‑endian base‑128 with the most significant bit indicating continuation.  To ensure
cross‑language consistency, Appendix N now lists concrete encodings for a selection of boundary values.  These
vectors can be used in test suites to verify varint encoders and decoders:

| value        | hex encoding |
|-------------:|:-------------|
| 0            | `00`         |
| 63           | `3f`         |
| 64           | `40`         |
| 16383        | `ff7f`       |
| 16384        | `808001`     |
| 2³° − 1      | `ffff07`     |
| 2³°          | `80808008`   |

The last two entries demonstrate multi‑byte encodings that extend into the fourth byte.  Any implementation
intending to support block identifiers larger than 2³° should treat these as illustrative only and extend the
table accordingly.

## Micro‑systematic REF example

In addition to the existing REF frame definition, Appendix N now includes an example of a **micro‑systematic** REF.
This mode seeds the block payload with a small fraction of the raw data (typically 5 %) before the parity and
rateless refinement.  The example shows a header with `block_id = 1`, `codex_slot = 2`, `lane = 1`, `flags = 2`
(indicating micro‑systematic), an arbitrary session tag of eight bytes, followed by a one‑byte micro‑seed (`05`),
the varint‑encoded seed length (`01`), the seed itself (a single raw byte `ff`) and a zero‑length TLV list.

```
01   # block_id = 1
12   # header byte: codex_slot=2, lane=1, flags=0b10 (micro-systematic)
00 00 00 00 00 00 00 00 # session tag (zero for example)
05   # micro‑systematic seed percentage (5%)
01   # varint‑encoded seed length = 1 byte
ff   # micro‑seed raw byte
00   # varint‑encoded TLV length = 0 (no TLVs)
00   # varint‑encoded ref payload length = 0 (empty for illustration)
```

## Extended ACK/NEED vectors

To exercise the new micro‑systematic mode, Appendix N adds companion ACK and NEED frames.  The ACK example
acknowledges completion of `block_id = 1` using a `block_id` varint (`01`) and no TLVs.  The NEED example
requests additional parity and includes a hint byte of `01` indicating “increase parity slope.”

```
# ACK_PASS for block_id = 1
01

# NEED (raise parity) for block_id = 1
01 01
```

## Normative video TLV layout

URDP‑X streams use Type/Length/Value (TLV) entries to annotate block headers with video‑specific metadata.  Each TLV
consists of a one‑byte `type`, a varint‑encoded `length`, and the `value` in big‑endian format where applicable.

| Type | Name          | Value format                                          |
|:----:|---------------|--------------------------------------------------------|
| 0x10 | `FrameTsUs`   | 8‑byte big‑endian microsecond timestamp               |
| 0x11 | `Ids`         | a sequence of varints: `gop_id`, `frame_id`, `temporal_id`, `spatial_id`, `layer_id` |
| 0x12 | `DeadlineMs`  | 2‑byte big‑endian decode deadline in milliseconds     |
| 0x13 | `Tile`        | two varints: `tile_id`, `tile_count`                  |

The TLV block appears immediately after the header and before the varint‑encoded reference length in a REF
frame.  TLVs must be processed in order; unknown TLV types SHOULD be ignored by forward‑compatible receivers.

### Example REF+TLV

The following hex stream shows a REF frame with one `FrameTsUs` TLV (type `0x10`) containing a timestamp of
`1 000 000 us`, followed by a `Ids` TLV with `gop_id = 7`, `frame_id = 42`, `temporal_id = 1`, `spatial_id = 0`,
`layer_id = 0`, and a zero‑length reference payload.

```
02        # block_id = 2
10        # header byte: codex_slot=0, lane=1, flags=0
00 00 00 00 00 00 00 01  # session tag = 1
10        # TLV type 0x10 (FrameTsUs)
08        # TLV length = 8
00 00 0f 42 40 00 00 00  # 1_000_000 us in big-endian
11        # TLV type 0x11 (Ids)
05        # TLV length = 5 bytes total
07        # gop_id
2a        # frame_id
01        # temporal_id
00        # spatial_id
00        # layer_id
00        # varint-encoded ref length = 0 (no reference data)
```

Implementers should refer to the Python hex‑checker tool (`tools/urdp_hexcheck.py`) included in this patch for
additional examples and for a reference parser.

---

This patch is intended to be applied on top of the existing URDP specification.  It defines concrete behaviour
required for interoperability and must not be altered without a corresponding version bump.
