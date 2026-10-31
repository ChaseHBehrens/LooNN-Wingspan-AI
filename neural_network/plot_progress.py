"""Plots training progress: training-game scores and the hourly tests against Randy.

Usage: py -3.12 neural_network/plot_progress.py [--show]
Saves logs/progress.png (and opens a window with --show).
"""
from pathlib import Path
import re
import sys
import matplotlib
if "--show" not in sys.argv:
    matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parent.parent
SCORES = ROOT / "neural_network" / "game_scores.csv"
PROGRESS = ROOT / "logs" / "progress.log"
OUT = ROOT / "logs" / "progress.png"
WINDOW = 400  # player-games in the rolling average
# Player-games when the random-action and bird-bonus schedules began (SCHEDULE_START_PLAYER_GAMES)
SCHEDULE_START = 9_101

scores = np.loadtxt(SCORES, delimiter=",", skiprows=1, usecols=1, ndmin=1)
x = np.arange(len(scores))

tests = []
if PROGRESS.exists():
    pattern = re.compile(r"player-games (\d+).*Randy ([\d.]+)\s+LooNN ([\d.]+)")
    for line in PROGRESS.read_text().splitlines():
        if m := pattern.search(line):
            tests.append(tuple(float(v) for v in m.groups()))

fig, ax = plt.subplots(figsize=(11, 6))
ax.scatter(x, scores, s=2, alpha=0.12, color="tab:blue", label="training game")
if len(scores) >= WINDOW:
    rolling = np.convolve(scores, np.ones(WINDOW) / WINDOW, mode="valid")
    ax.plot(x[WINDOW - 1:], rolling, color="tab:blue", lw=2,
            label=f"training average (last {WINDOW})")
if tests:
    t = np.array(tests)
    ax.plot(t[:, 0], t[:, 2], "o-", color="tab:green", lw=2, label="test: LooNN")
    ax.plot(t[:, 0], t[:, 1], "s--", color="tab:red", lw=1.5, label="test: Randy (random)")
ax.set_ylim(0, max(80, scores.max() + 5))
if len(scores) > SCHEDULE_START:
    ax.axvline(SCHEDULE_START, color="gray", ls=":", lw=1.5)
    ax.text(SCHEDULE_START, ax.get_ylim()[1] - 1, "random actions + bird bonus  ",
            color="gray", va="top", ha="right", fontsize=9)
ax.axhspan(60, 70, color="gold", alpha=0.15, label="goal (60-70)")
ax.set_xlabel("player-games trained")
ax.set_ylabel("points")
ax.set_title("LooNN training progress")
ax.grid(alpha=0.3)
ax.legend(loc="upper left", fontsize=9)
fig.tight_layout()
fig.savefig(OUT, dpi=120)
print(f"saved {OUT}")
if "--show" in sys.argv:
    plt.show()
