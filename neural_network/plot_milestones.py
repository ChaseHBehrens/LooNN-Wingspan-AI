"""Plots LooNN v0 training progress with a dotted line at each change to the setup.

Usage: py -3.12 neural_network/plot_milestones.py
Saves logs/progress_milestones.png.
"""
from pathlib import Path
import re
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parent.parent
SCORES = ROOT / "neural_network" / "game_scores.csv"
PROGRESS = ROOT / "logs" / "progress.log"
OUT = ROOT / "logs" / "progress_milestones.png"
WINDOW = 1_000  # player-games in the rolling average

# Player-games at each change, from logs/watchdog.log restarts matched to the score log
CHANGES = [
    (9_101, "random actions +\nearly bird bonus"),
    (52_520, "cost-vs-food features\n+ wilds fix"),
    (79_120, "learning rate halved"),
    (84_204, "bird bonus (fading)\n+ other changes"),
    (93_470, "rule fixes +\nposition fix"),
    (113_444, "bird bonus\n(fading, 2nd)"),
    (130_748, "permanent\n2-point bird bonus"),
]
# Second line drawn without its own label: the position fix landed ~400 games after the rule fixes
EXTRA_LINES = [93_860]
# Stretches where choices got a bird bonus (tests used it too)
BONUS_SPANS = [(84_204, 96_600), (113_444, 123_444)]

# Reference palette (dataviz skill): categorical slots 1-2, neutral ink and grid
BLUE, ORANGE = "#2a78d6", "#eb6834"
INK, INK_2, MUTED, GRID = "#0b0b0b", "#52514e", "#8a8984", "#e6e5e1"
SURFACE, GOAL, BONUS = "#fcfcfb", "#f6eccf", "#e8f0fa"

scores = np.loadtxt(SCORES, delimiter=",", skiprows=1, usecols=1, ndmin=1)
x = np.arange(len(scores))
rolling = np.convolve(scores, np.ones(WINDOW) / WINDOW, mode="valid")

tests = []
pattern = re.compile(r"player-games (\d+).*Randy ([\d.]+)\s+LooNN ([\d.]+)")
for line in PROGRESS.read_text().splitlines():
    if m := pattern.search(line):
        tests.append(tuple(float(v) for v in m.groups()))
t = np.array(tests)

plt.rcParams.update({"font.family": "sans-serif", "font.size": 10})
fig, ax = plt.subplots(figsize=(14, 7.5), facecolor=SURFACE)
ax.set_facecolor(SURFACE)

ax.axhspan(60, 70, color=GOAL, zorder=0)
ax.text(len(scores) * 0.004, 69.3, "goal 60-70", color=INK_2, fontsize=9, va="top", zorder=1)
for a, b in BONUS_SPANS:
    ax.axvspan(a, b, color=BONUS, zorder=0)
ax.axvspan(130_748, len(scores), color=BONUS, zorder=0)

ax.plot(x[WINDOW - 1:], rolling, color=BLUE, lw=2, zorder=4,
        label=f"LooNN training average (last {WINDOW:,} games)")
ax.plot(t[:, 0], t[:, 2], "o-", color=ORANGE, lw=1.5, ms=4.5, zorder=5,
        markeredgecolor=SURFACE, markeredgewidth=1, label="LooNN test vs Randy (30 games)")
ax.plot(t[:, 0], t[:, 1], "s--", color=INK_2, lw=1, ms=3.5, zorder=3,
        markeredgecolor=SURFACE, markeredgewidth=0.8, label="Randy (random player) in tests")

top = 100
for pos in [p for p, _ in CHANGES] + EXTRA_LINES:
    ax.axvline(pos, color=MUTED, ls=(0, (2, 3)), lw=1.2, zorder=2)
for pos, label in CHANGES:
    ax.text(pos + 600, top - 1.5, label, rotation=90, va="top", ha="left",
            color=INK, fontsize=8.5, linespacing=1.15, zorder=6,
            bbox=dict(facecolor=SURFACE, edgecolor="none", alpha=0.85, pad=1.5))

# Direct labels at the right edge for the three series
end = len(scores)
ax.annotate(f"{rolling[-1]:.1f}", (end, rolling[-1]), xytext=(8, -9), textcoords="offset points",
            color=INK, fontsize=9, fontweight="bold", va="center")
ax.annotate(f"{t[-1, 2]:.1f}", (t[-1, 0], t[-1, 2]), xytext=(8, 7), textcoords="offset points",
            color=INK_2, fontsize=9, va="center")
ax.annotate(f"{t[-1, 1]:.1f}", (t[-1, 0], t[-1, 1]), xytext=(6, 0), textcoords="offset points",
            color=INK_2, fontsize=9, va="center")

ax.set_xlim(0, end * 1.03)
ax.set_ylim(0, top)
ax.xaxis.set_major_formatter(matplotlib.ticker.FuncFormatter(lambda v, _: f"{v / 1000:.0f}k"))
ax.set_xlabel("player-games trained", color=INK_2)
ax.set_ylabel("points per game", color=INK_2)
ax.grid(axis="y", color=GRID, lw=0.8)
ax.set_axisbelow(True)
for side in ("top", "right"):
    ax.spines[side].set_visible(False)
for side in ("left", "bottom"):
    ax.spines[side].set_color(MUTED)
ax.tick_params(colors=INK_2)

ax.set_title("LooNN v0 training progress", loc="left", fontsize=15, color=INK, pad=24, fontweight="bold")
ax.text(0, 1.015, "Dotted lines mark changes to the setup; light blue spans had a bird bonus on choices (tests too).",
        transform=ax.transAxes, color=INK_2, fontsize=9.5)
legend = ax.legend(loc="lower right", frameon=True, fontsize=9)
legend.get_frame().set_facecolor(SURFACE)
legend.get_frame().set_edgecolor(GRID)

fig.tight_layout()
fig.savefig(OUT, dpi=150, facecolor=SURFACE)
print(f"saved {OUT}")
