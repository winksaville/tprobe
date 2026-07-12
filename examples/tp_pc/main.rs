//! tp_pc — parity port of iiac-perf's `tp-pc` bench onto the
//! tprobe crate's collection phase.
//!
//! A dedicated producer thread and a dedicated consumer thread
//! trade messages over two `std::sync::mpsc` channels; each
//! actor measures its own full loop iteration. Identical loop,
//! channels, and band-table output to iiac-perf's `tp-pc`; what
//! differs is the recording:
//!
//! - iiac-perf `tp-pc`: hdrhistogram `record()` per iteration
//!   (compress at collection).
//! - here: `TProbe` + [`ArrayRecorder`] — one raw-delta store
//!   per iteration; the histogram is built *after* the run
//!   (analyze off the measured path, per `notes/design.md`).
//!
//! Run back-to-back with iiac-perf's `tp-pc` on the same cores
//! to see the collection-phase cost/benefit.

mod band_table;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use hdrhistogram::Histogram;
use tprobe::{ArrayRecorder, TProbe, ticks};

/// Histogram bounds matching iiac-perf's probes: 1..1e12 ticks
/// (~250 s at 4 GHz), 3 significant figures.
const HIST_HIGH: u64 = 1_000_000_000_000;

/// Default per-thread delta capacity: 8 Mi samples (64 MiB).
const DEFAULT_CAP: usize = 8 * 1024 * 1024;

/// Parsed CLI configuration.
struct Cfg {
    /// Measured run duration in seconds.
    secs: f64,
    /// Logical CPUs for (producer, consumer); `None` = unpinned.
    cores: [Option<usize>; 2],
    /// Per-thread delta-buffer capacity, in samples.
    cap: usize,
    /// Report raw ticks instead of nanoseconds.
    as_ticks: bool,
}

/// Parse CLI args. `Ok(None)` means `--help` was printed.
fn parse_args(mut args: std::env::Args) -> Result<Option<Cfg>, String> {
    let mut cfg = Cfg {
        secs: 5.0,
        cores: [None, None],
        cap: DEFAULT_CAP,
        as_ticks: false,
    };
    args.next(); // skip argv[0]
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--secs" => {
                let v = args.next().ok_or("--secs needs a value")?;
                cfg.secs = v
                    .parse::<f64>()
                    .map_err(|e| format!("invalid --secs {v:?}: {e}"))?;
            }
            "--cores" => {
                let v = args.next().ok_or("--cores needs a value, e.g. 0,1")?;
                let mut it = v.split(',');
                let (Some(a), Some(b), None) = (it.next(), it.next(), it.next()) else {
                    return Err(format!("--cores wants exactly two ids, got {v:?}"));
                };
                cfg.cores = [
                    Some(a.parse().map_err(|e| format!("invalid core {a:?}: {e}"))?),
                    Some(b.parse().map_err(|e| format!("invalid core {b:?}: {e}"))?),
                ];
            }
            "--cap" => {
                let v = args.next().ok_or("--cap needs a value")?;
                cfg.cap = v
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --cap {v:?}: {e}"))?;
            }
            "--ticks" | "-t" => cfg.as_ticks = true,
            "--help" | "-h" => {
                println!(
                    "usage: tp_pc [--secs F] [--cores A,B] [--cap N] [--ticks]\n\
                     \n\
                     defaults: --secs 5.0, unpinned, --cap {DEFAULT_CAP} (per thread)"
                );
                return Ok(None);
            }
            other => return Err(format!("unknown arg {other:?} (try --help)")),
        }
    }
    Ok(Some(cfg))
}

/// Pin the current thread to `logical_cpu`. No-op if `None`.
fn pin_current(logical_cpu: Option<usize>) {
    let Some(target) = logical_cpu else { return };
    core_affinity::set_for_current(core_affinity::CoreId { id: target });
}

/// Build a probe over a preallocated, pre-faulted delta buffer,
/// so first-touch page faults don't land inside the measured
/// loop.
fn make_probe(name: &'static str, cap: usize) -> TProbe<ArrayRecorder<Vec<u64>>> {
    let mut buf = vec![0u64; cap];
    // Touch one u64 per 4 KiB page.
    buf.iter_mut().step_by(512).for_each(|v| *v = 1);
    std::hint::black_box(&mut buf);
    TProbe::new(name, ArrayRecorder::new(buf))
}

/// Analysis + presentation, after the run: build the histogram
/// the iiac-perf variant builds on the hot path, then render the
/// identical band table.
fn report(name: &str, rec: &ArrayRecorder<Vec<u64>>, as_ticks: bool) {
    #[allow(clippy::unwrap_used)]
    // OK: bounds are valid constants (1 ≤ low < high, sigfig 3 ≤ 5)
    let mut hist = Histogram::<u64>::new_with_bounds(1, HIST_HIGH, 3).unwrap();
    for &d in rec.deltas() {
        #[allow(clippy::unwrap_used)]
        // OK: value clamped into the histogram's bounds
        hist.record(d.clamp(1, HIST_HIGH)).unwrap();
    }
    band_table::render("tprobe", name, &hist, as_ticks);
    if rec.dropped() > 0 {
        println!(
            "    WARNING: {} deltas dropped (buffer full; raise --cap)",
            band_table::fmt_commas(rec.dropped())
        );
    }
}

/// Bench entry point: spawn the pinned producer/consumer pair,
/// run for `--secs`, then analyze and report.
fn main() {
    let cfg = match parse_args(std::env::args()) {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return,
        Err(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(2);
        }
    };
    if let Err(msg) = ticks::check() {
        eprintln!("error: {msg}; refusing to run");
        std::process::exit(1);
    }
    // Trigger calibration eagerly so the report doesn't pay for it.
    let _ = ticks::ticks_per_ns();

    let (req_tx, req_rx) = mpsc::channel::<u64>();
    let (resp_tx, resp_rx) = mpsc::channel::<u64>();
    let shutdown = Arc::new(AtomicBool::new(false));

    let [producer_cpu, consumer_cpu] = cfg.cores;
    let cap = cfg.cap;

    let producer_shutdown = shutdown.clone();
    let producer = thread::spawn(move || {
        pin_current(producer_cpu);
        let mut probe = make_probe("producer loop", cap);
        let mut counter: u64 = 0;
        while !producer_shutdown.load(Ordering::Relaxed) {
            probe.start();
            counter = counter.wrapping_add(1);
            if req_tx.send(counter).is_err() {
                break;
            }
            if resp_rx.recv().is_err() {
                break;
            }
            probe.record();
        }
        probe
    });

    let consumer = thread::spawn(move || {
        pin_current(consumer_cpu);
        let mut probe = make_probe("consumer loop", cap);
        loop {
            probe.start();
            let v = match req_rx.recv() {
                Ok(v) => v,
                Err(_) => break,
            };
            if resp_tx.send(v).is_err() {
                break;
            }
            probe.record();
        }
        probe
    });

    thread::sleep(Duration::from_secs_f64(cfg.secs));
    shutdown.store(true, Ordering::Relaxed);

    #[allow(clippy::expect_used)]
    // OK: propagate a child-thread panic; fail-fast is correct in a bench
    let producer_probe = producer.join().expect("producer panicked");
    #[allow(clippy::expect_used)]
    // OK: propagate a child-thread panic; fail-fast is correct in a bench
    let consumer_probe = consumer.join().expect("consumer panicked");

    println!(
        "tp_pc (2 threads, tprobe ArrayRecorder) [duration={:.1}s]:",
        cfg.secs
    );
    report(
        producer_probe.name(),
        producer_probe.recorder(),
        cfg.as_ticks,
    );
    report(
        consumer_probe.name(),
        consumer_probe.recorder(),
        cfg.as_ticks,
    );
}
