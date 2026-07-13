#!/usr/bin/env bash
# tp-pc comparison driver: A = iiac-perf's tp-pc (hdrhistogram
# at collection), B = examples/tp_pc (tprobe ArrayRecorder).
# Findings and protocol rationale live in
# notes/chores/chores-01.md ("tp_pc parity comparison").
#
# - Run from the tprobe repo root; expects an iiac-perf checkout
#   as a sibling (../iiac-perf), both built --release.
# - Starts with a discarded 60 s warmup (first run of a session
#   is not usable — round-1 lesson).
# - B's --cap is sized to need from an upper-bound rate estimate
#   (~140 K samples/s), not padded (round-2/3 lesson: a fresh
#   oversized buffer is a run-to-run variance suspect).
# - Output tees to ./tmp/ (ignored); promote a run worth keeping
#   into notes/ as a lab record.
# - A runs with --no-inhibit: symmetric with B (which has no
#   inhibitor), and iiac-perf's default systemd-inhibit re-exec is
#   denied by polkit in an ssh session. Sleep inhibition is the
#   operator's job — arm it from a desktop session before a sweep:
#   systemd-inhibit --what=sleep:idle --why=sweep sleep 7200
#
# usage: scripts/tp-pc-cmp.sh abba  [secs] [cores]
#        scripts/tp-pc-cmp.sh pairs [secs] [npairs] [cores]
# defaults: abba 300 s; pairs 60 s x 10; cores 2,3.
# standard cell: pairs 60 10 (resolves ~50 ns); smoke: pairs 6 10.
set -eu

IIAC=../iiac-perf/target/release/iiac-perf
TP=./target/release/examples/tp_pc

usage() {
    grep '^# usage' -A 3 "$0" | sed 's/^# //' >&2
    exit 2
}

[ -x "$IIAC" ] || {
    echo "error: $IIAC missing (cargo build --release in ../iiac-perf)" >&2
    exit 1
}
[ -x "$TP" ] || {
    echo "error: $TP missing (cargo build --release --example tp_pc)" >&2
    exit 1
}

# A killed ssh session can leave a prior sweep running (its tee
# writes host-side); overlapping runs contend on the pinned cores.
if pgrep 'iiac-perf|tp_pc' > /dev/null; then
    echo "error: a benchmark is already running (pgrep 'iiac-perf|tp_pc')" >&2
    exit 1
fi

mode=${1:-}
case "$mode" in
    abba)
        secs=${2:-300}
        cores=${3:-2,3}
        ;;
    pairs)
        secs=${2:-60}
        npairs=${3:-10}
        cores=${4:-2,3}
        ;;
    *) usage ;;
esac
# Upper-bound samples/s for --cap sizing. Pair-dependent on the
# 3900x: cross-CCX (2,3) ~115–126 K/s, same-CCX (4,5) ~198 K/s;
# 220 K covers both with margin. A too-small cap truncates B's
# recording (deltas dropped, count clamped at cap — throughput
# metric lost); at 8 B/sample the pad stays ~100 MB at 60 s,
# below the 400 MB round-2 variance concern.
rate=220000
cap=$((secs * rate))

run_a() { echo "=== $1: iiac-perf tp-pc ($(date +%T)) ==="; "$IIAC" --no-inhibit tp-pc --duration "$secs" --pin "$cores"; }
run_b() { echo "=== $1: tprobe tp_pc ($(date +%T)) ==="; "$TP" --secs "$secs" --cores "$cores" --cap "$cap"; }

main() {
    echo "=== config: host=$(hostname) mode=$mode secs=$secs npairs=${npairs:-} cores=$cores cap=$cap ==="
    echo "=== WARMUP (discard): iiac-perf tp-pc 60s ($(date +%T)) ==="
    "$IIAC" --no-inhibit tp-pc --duration 60 --pin "$cores" > /dev/null 2>&1

    case "$mode" in
        abba)
            run_a A1
            run_b B1
            run_b B2
            run_a A2
            ;;
        pairs)
            for i in $(seq 1 "$npairs"); do
                run_a "A$i"
                run_b "B$i"
            done
            ;;
    esac
    echo "=== done ($(date +%T)) ==="
}

mkdir -p tmp
out="tmp/tp-pc-cmp-$mode-${secs}s-$(date +%Y%m%d-%H%M%S).txt"
echo "writing $out"
main 2>&1 | tee "$out"
