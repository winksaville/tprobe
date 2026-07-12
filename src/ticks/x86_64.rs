//! x86_64 impl of the tick-counter abstraction: `rdtsc` for
//! reads (`no_std`), CPUID-based invariant-TSC detection, and a
//! 10 ms spin-loop calibration for ticks-per-nanosecond (`std`).

/// Read the TSC. Safe on any x86_64 CPU: the TSC has been
/// present since the original Pentium.
#[inline(always)]
pub fn read_ticks() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[cfg(feature = "std")]
static TICKS_PER_NS: std::sync::OnceLock<f64> = std::sync::OnceLock::new();

/// Ticks per nanosecond, calibrated once and cached.
#[cfg(feature = "std")]
pub fn ticks_per_ns() -> f64 {
    *TICKS_PER_NS.get_or_init(calibrate)
}

/// Spin for ~10 ms reading `std::time::Instant` elapsed ns and
/// raw `rdtsc` ticks at each end, and derive the ratio from the
/// two independent measurements. (iiac-perf calibrates against
/// `minstant`; `std::time::Instant` is `clock_gettime(MONOTONIC)`
/// — ns-resolution, ample over a 10 ms window — and drops the
/// dependency.)
#[cfg(feature = "std")]
fn calibrate() -> f64 {
    let start_instant = std::time::Instant::now();
    let start_tsc = read_ticks();
    let target = std::time::Duration::from_millis(10);
    loop {
        let elapsed = start_instant.elapsed();
        if elapsed >= target {
            let end_tsc = read_ticks();
            let dtk = end_tsc.wrapping_sub(start_tsc) as f64;
            let dns = elapsed.as_nanos() as f64;
            return dtk / dns;
        }
        core::hint::spin_loop();
    }
}

/// Verify the TSC is usable for probe measurements:
///
/// - the CPU advertises an invariant TSC, and
/// - (Linux) the kernel selected `tsc` as its clocksource — a
///   CPU-advertised-but-kernel-rejected TSC usually means a sync
///   or drift issue this library shouldn't measure with.
#[cfg(feature = "std")]
pub fn check() -> Result<(), &'static str> {
    if !has_invariant_tsc() {
        return Err("invariant TSC not supported by this CPU \
             (CPUID.80000007h:EDX[bit 8] = 0); tprobe requires a \
             fixed-rate, non-stopping TSC");
    }
    #[cfg(target_os = "linux")]
    if !kernel_clocksource_is_tsc() {
        return Err("TSC not selected as the kernel clocksource — the CPU \
             advertises invariant TSC, but the kernel has rejected \
             it (likely a sync or drift issue)");
    }
    Ok(())
}

/// `CPUID.80000007h:EDX[bit 8]` — invariant TSC. Set iff the
/// TSC runs at a constant rate regardless of P-state changes
/// and keeps ticking in deep C-states. Both Intel and AMD
/// expose the feature at this bit.
#[cfg(feature = "std")]
fn has_invariant_tsc() -> bool {
    use core::arch::x86_64::__cpuid;
    let max_ext = __cpuid(0x8000_0000).eax;
    if max_ext < 0x8000_0007 {
        return false;
    }
    let leaf = __cpuid(0x8000_0007);
    (leaf.edx >> 8) & 1 == 1
}

/// Whether the kernel's current clocksource is the TSC, read
/// from sysfs. An unreadable sysfs (containers, non-standard
/// kernels) counts as "not tsc" — refusing is the safe default.
#[cfg(all(feature = "std", target_os = "linux"))]
fn kernel_clocksource_is_tsc() -> bool {
    std::fs::read_to_string("/sys/devices/system/clocksource/clocksource0/current_clocksource")
        .is_ok_and(|s| s.trim() == "tsc")
}
