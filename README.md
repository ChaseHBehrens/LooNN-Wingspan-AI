[Play with LooNN](https://chasehbehrens.github.io/LooNN-Wingspan-AI/)

## Terminal version

Play a real game of Wingspan with LooNN in one seat from your terminal: you enter the cards and dice that come out, LooNN prints its moves, and you make them on the table for it.

### Download

Get the build for your system from the [latest release](https://github.com/ChaseHBehrens/LooNN-Wingspan-AI/releases/latest), unpack it, and run `loonn-terminal` from a terminal:

| System | File |
| --- | --- |
| Windows | `loonn-terminal-windows-x86_64.zip` |
| macOS (Apple Silicon) | `loonn-terminal-macos-arm64.tar.gz` |
| macOS (Intel) | `loonn-terminal-macos-x86_64.tar.gz` |
| Linux | `loonn-terminal-linux-x86_64.tar.gz` |

On macOS and Linux:

```bash
tar -xzf loonn-terminal-macos-arm64.tar.gz
./loonn-terminal-macos-arm64/loonn-terminal
```

macOS blocks downloaded programs that aren't signed. If it says the program can't be opened, clear the download flag once:

```bash
xattr -d com.apple.quarantine loonn-terminal-macos-arm64/loonn-terminal
```

### Run from source

With [Rust](https://rustup.rs) installed:

```bash
cargo run --release --no-default-features --features terminal --bin terminal
```

`cargo run --release --bin terminal` works too, but it also builds the PyTorch bridge, which needs Python 3.12 with PyTorch. That build accepts `--pytorch` to use the training checkpoint directly instead of the exported network in `web/model.bin`.

### Playing

- **Setup:** pick the 4 round goals, then the 5 birds and 2 bonus cards dealt to LooNN. Type to search the lists.
- **Each turn:** once the other players have gone, tick anything they did that LooNN's birds react to, then press Enter. LooNN asks for whatever it needs: the dice in the birdfeeder, the face-up birds, or the card drawn from the deck.
- **Dice:** type them separated by spaces: `i` invertebrate, `s` seed, `f` fish, `fr` fruit, `r` rodent, `is` invertebrate/seed.
- **LooNN's moves** are printed after each turn, followed by its board, score, and predicted final score.
- Press Esc or Ctrl+C to quit.
