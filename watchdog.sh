#!/usr/bin/env bash
# Runs training unattended: restarts it if it crashes, and every hour plays
# LooNN against Randy and records progress.
#
# Usage:   ./watchdog.sh [parallel games]
# Detach:  nohup ./watchdog.sh > /dev/null 2>&1 &
# Stop:    kill "$(cat logs/watchdog.pid)"   (also stops training)
#
# Logs (in logs/):
#   train.log     everything training prints
#   progress.log  one line per hour: games, recent average, exploration, test vs Randy, disk
#   watchdog.log  starts, crashes and restarts

set -u
cd "$(dirname "$0")"

PARALLEL_GAMES="${1:-}"
EVAL_EVERY=3600       # seconds between progress checks
EVAL_GAMES=30         # full test games per progress check (~3 minutes)
MAX_FAST_CRASHES=5    # give up after this many crashes in a row...
FAST_CRASH_SECONDS=300 # ...that each happen within this long of starting

mkdir -p logs
echo $$ > logs/watchdog.pid
log() { echo "$(date '+%F %T') $*" >> logs/watchdog.log; }

train_pid=""
stop() {
    log "watchdog stopping"
    [ -n "$train_pid" ] && kill "$train_pid" 2>/dev/null
    rm -f logs/watchdog.pid
    exit 0
}
trap stop INT TERM

log "building"
if ! cargo build --release --bins >> logs/watchdog.log 2>&1; then
    log "build failed, see above"
    exit 1
fi

start_training() {
    PYTHONUNBUFFERED=1 ./target/release/train $PARALLEL_GAMES >> logs/train.log 2>&1 &
    train_pid=$!
    train_started=$(date +%s)
    log "training started (pid $train_pid)"
}

progress() {
    local scores=neural_network/game_scores.csv games average exploration result disk
    games=$(( $(wc -l < "$scores" 2>/dev/null || echo 1) - 1 ))
    average=$(tail -n 200 "$scores" 2>/dev/null | grep -v '^game' \
        | awk -F, '{ s += $2; n++ } END { if (n) printf "%.1f", s / n; else print "-" }')
    exploration=$(grep -o 'exploration [0-9.]*' logs/train.log 2>/dev/null | tail -n 1 | cut -d' ' -f2)
    # Plays against the latest saved checkpoint while training continues
    result=$(./target/release/test full "$EVAL_GAMES" 2>&1 | tr '\r' '\n' \
        | grep -E 'averaged' | awk '{ printf "%s %s  ", $1, $3 }')
    disk=$(df -h . | awk 'NR == 2 { print $4 }')
    printf '%s  player-games %s  avg(last 200) %s  exploration %s  test(%s games): %s disk free %s\n' \
        "$(date '+%F %T')" "$games" "$average" "${exploration:--}" "$EVAL_GAMES" "$result" "$disk" \
        >> logs/progress.log
}

start_training
fast_crashes=0
next_eval=$(( $(date +%s) + EVAL_EVERY ))
while true; do
    sleep 30
    if ! kill -0 "$train_pid" 2>/dev/null; then
        wait "$train_pid"
        status=$?
        ran=$(( $(date +%s) - train_started ))
        log "training exited with status $status after ${ran}s; last lines of train.log:"
        tail -n 15 logs/train.log >> logs/watchdog.log
        if [ "$ran" -lt "$FAST_CRASH_SECONDS" ]; then
            fast_crashes=$(( fast_crashes + 1 ))
        else
            fast_crashes=0
        fi
        if [ "$fast_crashes" -ge "$MAX_FAST_CRASHES" ]; then
            log "crashed $fast_crashes times in a row within ${FAST_CRASH_SECONDS}s of starting; giving up"
            rm -f logs/watchdog.pid
            exit 1
        fi
        sleep 10
        start_training
    fi
    if [ "$(date +%s)" -ge "$next_eval" ]; then
        progress
        next_eval=$(( $(date +%s) + EVAL_EVERY ))
    fi
done
