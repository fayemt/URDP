#!/usr/bin/env python3
"""
URDP hex checker.

This script decodes URDP frames from hexadecimal strings and prints a
human‑readable representation.  It supports REF, REF+TLV, PARITY, ACK_PASS
and NEED frames.  Its primary purpose is to validate encoder output and
illustrate the wire format.

Usage:
    python urdp_hexcheck.py <type> <hexstring>
    python urdp_hexcheck.py               # runs a built‑in demo

<type> may be one of: ref, parity, ack, need.  If omitted the script
runs a demo using example hex taken from the specification.
"""
import sys
from typing import Tuple, List


def decode_varint(data: bytes, offset: int = 0) -> Tuple[int, int]:
    """Decode a little‑endian base‑128 varint from ``data`` starting at ``offset``.

    Returns a tuple of the value and the number of bytes consumed.
    """
    value = 0
    shift = 0
    i = offset
    while i < len(data):
        byte = data[i]
        value |= (byte & 0x7F) << shift
        i += 1
        if (byte & 0x80) == 0:
            break
        shift += 7
    return value, i - offset


def decode_header(data: bytes) -> Tuple[int, int, int, int, bytes, int]:
    """Decode a URDP block header.

    Returns a tuple ``(block_id, codex_slot, lane, flags, session_tag, bytes_consumed)``.
    """
    block_id, consumed = decode_varint(data)
    if len(data) < consumed + 1 + 8:
        raise ValueError("header truncated")
    hdr = data[consumed]
    codex_slot = hdr & 0x0F
    lane = (hdr >> 4) & 0x03
    flags = (hdr >> 6) & 0x03
    tag = data[consumed + 1:consumed + 9]
    return block_id, codex_slot, lane, flags, tag, consumed + 1 + 8


def decode_tlvs(data: bytes) -> Tuple[List[Tuple[int, bytes]], int]:
    """Decode a sequence of TLVs from ``data``.

    Returns a list of ``(type, value)`` pairs and the number of bytes consumed.
    """
    tlvs = []
    offset = 0
    while offset < len(data):
        t = data[offset]
        offset += 1
        length, nlen = decode_varint(data, offset)
        offset += nlen
        val = data[offset:offset + length]
        offset += length
        tlvs.append((t, val))
        if length == 0:
            break
    return tlvs, offset


def parse_ref(hexstr: str) -> dict:
    data = bytes.fromhex(hexstr)
    block_id, slot, lane, flags, tag, off = decode_header(data)
    result = {
        "block_id": block_id,
        "codex_slot": slot,
        "lane": lane,
        "flags": flags,
        "session_tag": tag.hex(),
    }
    # TLV section length
    try:
        tlv_len, nlen = decode_varint(data, off)
        off += nlen
        tlv_data = data[off:off + tlv_len]
        tlvs, _ = decode_tlvs(tlv_data)
        result["tlvs"] = [(t, v.hex()) for t, v in tlvs]
        off += tlv_len
    except Exception:
        result["tlvs"] = []
    # reference payload
    try:
        ref_len, nlen2 = decode_varint(data, off)
        off += nlen2
        ref = data[off:off + ref_len]
        result["ref_bytes"] = ref.hex()
    except Exception:
        result["ref_bytes"] = ""
    return result


def parse_parity(hexstr: str) -> dict:
    data = bytes.fromhex(hexstr)
    block_id, slot, lane, flags, tag, off = decode_header(data)
    return {
        "block_id": block_id,
        "codex_slot": slot,
        "lane": lane,
        "flags": flags,
        "session_tag": tag.hex(),
        "parity_bytes": data[off:].hex(),
    }


def parse_ack(hexstr: str) -> dict:
    data = bytes.fromhex(hexstr)
    block_id, _ = decode_varint(data)
    return {"block_id": block_id}


def parse_need(hexstr: str) -> dict:
    data = bytes.fromhex(hexstr)
    block_id, consumed = decode_varint(data)
    hint = data[consumed] if consumed < len(data) else 0
    return {"block_id": block_id, "hint": hint}


def demo():
    """Run a simple demonstration with built‑in vectors from the spec."""
    ref = "01 10 00 00 00 00 00 00 00 00 00 00".replace(" ", "")
    ref_tlv = (
        "02 10 00 00 00 00 00 00 00 01 "
        "10 08 00000f4240000000 "
        "11 05 07 2a 01 00 00 "
        "00"
    ).replace(" ", "")
    ack = "01"
    need = "01 01".replace(" ", "")
    for name, hexstr, parser in [
        ("REF (empty)", ref, parse_ref),
        ("REF+TLV", ref_tlv, parse_ref),
        ("ACK_PASS", ack, parse_ack),
        ("NEED", need, parse_need),
    ]:
        print(f"{name}:")
        print(parser(hexstr))
        print()


def main():
    if len(sys.argv) == 1:
        demo()
        return
    if len(sys.argv) != 3:
        print(__doc__.strip())
        return
    typ, hexstr = sys.argv[1], sys.argv[2]
    parsers = {
        "ref": parse_ref,
        "parity": parse_parity,
        "ack": parse_ack,
        "need": parse_need,
    }
    if typ not in parsers:
        print(f"unknown type {typ}")
        return
    result = parsers[typ](hexstr)
    print(result)


if __name__ == "__main__":
    main()
