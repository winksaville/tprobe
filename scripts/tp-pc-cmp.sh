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
#
# usage: scripts/tp-pc-cmp.sh abba  [secs] [cores]
#        scripts/tp-pc-cmp.sh pairs [secs] [npairs] [cores]
# defaults: abba 300 s; pairs 60 s x 10; cores 2,3.
set -eu

IIAC=../iiac-perf/target/release/iiac-perf
TP=./target/release/examples/tp_pc

usage() {
    grep '^# usage' -A 2 "$0" | sed 's/^# //' >&2
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
cap=$((secs * 140000))

run_a() { echo "=== $1: iiac-perf tp-pc ($(date +%T)) ==="; "$IIAC" tp-pc --duration "$secs" --pin "$cores"; }
run_b() { echo "=== $1: tprobe tp_pc ($(date +%T)) ==="; "$TP" --secs "$secs" --cores "$cores" --cap "$cap"; }

main() {
    echo "=== WARMUP (discard): iiac-perf tp-pc 60s ($(date +%T)) ==="
    "$IIAC" tp-pc --duration 60 --pin "$cores" > /dev/null 2>&1

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
