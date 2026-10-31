"""Exports the trained network's weights for the native Rust evaluator
(src/evaluators/native.rs), which the web version of the play page runs.

Usage: py -3.12 neural_network/export_weights.py [output]   (default: web/model.bin)

Format: b"LNN1", a u32 tensor count, then for each tensor a u32 name length,
the UTF-8 name, a u32 dimension count, u32 dimensions, and little-endian f32 values.
"""
from pathlib import Path
import struct
import sys

import torch

HERE = Path(__file__).resolve().parent
CHECKPOINT = HERE / "model_hand_attn.pt"
ARCH = "hand_attn_v3"


def main() -> None:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else HERE.parent / "web" / "model.bin"
    checkpoint = torch.load(CHECKPOINT, map_location="cpu", weights_only=False)
    if checkpoint.get("arch") != ARCH:
        sys.exit(f"{CHECKPOINT} is {checkpoint.get('arch')}, but native.rs implements {ARCH}")
    state = checkpoint["model_state"]
    with open(out, "wb") as f:
        f.write(b"LNN1")
        f.write(struct.pack("<I", len(state)))
        for name, tensor in state.items():
            data = tensor.detach().to(torch.float32).contiguous().numpy()
            encoded = name.encode()
            f.write(struct.pack("<I", len(encoded)))
            f.write(encoded)
            f.write(struct.pack("<I", data.ndim))
            f.write(struct.pack(f"<{data.ndim}I", *data.shape))
            f.write(data.astype("<f4").tobytes())
    games = checkpoint.get("train_call_count", 0)
    print(f"Wrote {out} ({out.stat().st_size / 1e6:.1f} MB, {games} training calls)")


if __name__ == "__main__":
    main()
