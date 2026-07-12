//! Band-table renderer for tick-valued histograms — a copy of
//! iiac-perf's `band_table.rs` (plus its `fmt_commas` helpers)
//! so `tp_pc` output is line-for-line comparable with the
//! iiac-perf `tp-pc` bench.
//!
//! Lives in the example, not the crate: presentation is a
//! deliberate non-goal of the core until the `no_std` histogram
//! question (Q1 in `notes/design.md`) is decided, and this copy
//! is discardable once the crate grows its own `std` reporting.

use hdrhistogram::Histogram;

use tprobe::ticks;

const BOUNDARY_PCTS: &[f64] = &[
    0.0, 0.01, 0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80, 0.90, 0.99, 1.0,
];
const BOUNDARY_NAMES: &[&str] = &[
    "min", "p1", "p10", "p20", "p30", "p40", "p50", "p60", "p70", "p80", "p90", "p99", "max",
];

/// Render a band-table report for `hist`, interpreting stored
/// values as hardware ticks. `kind` is the header label and
/// `name` is the probe's name. `as_ticks=false` converts to ns;
/// `true` keeps raw ticks.
pub fn render(kind: &str, name: &str, hist: &Histogram<u64>, as_ticks: bool) {
    let sample_count = hist.len();
    println!("  {kind}: {name} [count={}]", fmt_commas(sample_count));
    if sample_count == 0 {
        println!();
        return;
    }

    let unit = if as_ticks { "tk" } else { "ns" };
    let tpn = ticks::ticks_per_ns();
    let conv = |v: u64| -> f64 { if as_ticks { v as f64 } else { v as f64 / tpn } };
    let conv_f = |v: f64| -> f64 { if as_ticks { v } else { v / tpn } };

    let n_bands = BOUNDARY_PCTS.len() - 1;
    // Index of the top (p99-max) band: overflow target for
    // mid-ranks past the last boundary, excluded by the trimmed
    // (min-p99) summary stats.
    let top_band_idx = n_bands - 1;
    let mut band_first = vec![u64::MAX; n_bands];
    let mut band_last = vec![0u64; n_bands];
    let mut band_count = vec![0u64; n_bands];
    let mut band_sum = vec![0u128; n_bands];

    let mut cumulative = 0u64;
    for iv in hist.iter_recorded() {
        let value = iv.value_iterated_to();
        let count = iv.count_at_value();
        let mid_rank = (cumulative as f64 + count as f64 / 2.0) / sample_count as f64;
        let idx = BOUNDARY_PCTS[1..]
            .iter()
            .position(|&b| mid_rank < b)
            .unwrap_or(top_band_idx); // OK: mid_rank ≥ last boundary → top band
        band_first[idx] = band_first[idx].min(value);
        band_last[idx] = band_last[idx].max(value);
        band_count[idx] += count;
        band_sum[idx] += value as u128 * count as u128;
        cumulative += count;
    }

    struct BandRow {
        label: String,
        first: String,
        last: String,
        range: String,
        count: String,
        mean: String,
    }

    let mut rows: Vec<BandRow> = Vec::new();
    for i in 0..n_bands {
        if band_count[i] == 0 {
            continue;
        }
        let mean_val = band_sum[i] as f64 / band_count[i] as f64;
        let range_raw = band_last[i] - band_first[i] + 1;
        rows.push(BandRow {
            label: format!("{}-{}", BOUNDARY_NAMES[i], BOUNDARY_NAMES[i + 1]),
            first: fmt_commas_f64(conv(band_first[i]), 0),
            last: fmt_commas_f64(conv(band_last[i]), 0),
            range: fmt_commas_f64(conv(range_raw), 0),
            count: fmt_commas(band_count[i]),
            mean: fmt_commas_f64(conv_f(mean_val), 0),
        });
    }

    // Widest rendered cell in a column; 0 when rows is empty.
    let width = |cell: fn(&BandRow) -> &str| -> usize {
        rows.iter().map(|r| cell(r).len()).fold(0, usize::max)
    };
    let label_w = width(|r| r.label.as_str()).max("stdev min-p99".len());
    let first_w = width(|r| r.first.as_str());
    let last_w = width(|r| r.last.as_str());
    let range_w = width(|r| r.range.as_str());
    let count_w = width(|r| r.count.as_str());
    let mean_w = width(|r| r.mean.as_str());

    const INDENT: &str = "    ";
    const GAP: &str = "    ";

    let first_col = INDENT.len() + label_w + 1 + first_w;
    let unit_len = 1 + unit.len();
    let last_gap = unit_len + GAP.len() + last_w;
    let range_gap = unit_len + GAP.len() + range_w;
    let count_gap = unit_len + GAP.len() + count_w;
    let mean_gap = GAP.len() + mean_w;
    println!(
        "{:>first_col$}{:>last_gap$}{:>range_gap$}{:>count_gap$}{:>mean_gap$}",
        "first", "last", "range", "count", "mean",
    );

    for r in &rows {
        println!(
            "{INDENT}{:<label_w$} {:>first_w$} {unit}{GAP}{:>last_w$} {unit}{GAP}{:>range_w$} {unit}{GAP}{:>count_w$}{GAP}{:>mean_w$} {unit}",
            r.label, r.first, r.last, r.range, r.count, r.mean,
        );
    }

    let hist_mean = hist.mean();
    let skip = first_w
        + unit_len
        + GAP.len()
        + last_w
        + unit_len
        + GAP.len()
        + range_w
        + unit_len
        + GAP.len()
        + count_w;
    println!(
        "{INDENT}{:<label_w$} {:>skip$}{GAP}{:>mean_w$} {unit}",
        "mean",
        "",
        fmt_commas_f64(conv_f(hist_mean), 0),
    );
    println!(
        "{INDENT}{:<label_w$} {:>skip$}{GAP}{:>mean_w$} {unit}",
        "stdev",
        "",
        fmt_commas_f64(conv_f(hist.stdev()), 0),
    );

    let trim_count: u64 = band_count[..top_band_idx].iter().sum();
    if trim_count > 0 {
        let trim_sum: u128 = band_sum[..top_band_idx].iter().sum();
        let trim_mean = trim_sum as f64 / trim_count as f64;

        let mut trim_var_sum = 0.0f64;
        let mut trim_var_count = 0u64;
        let mut cum = 0u64;
        for iv in hist.iter_recorded() {
            let value = iv.value_iterated_to();
            let count = iv.count_at_value();
            let mid_rank = (cum as f64 + count as f64 / 2.0) / sample_count as f64;
            let idx = BOUNDARY_PCTS[1..]
                .iter()
                .position(|&b| mid_rank < b)
                .unwrap_or(top_band_idx); // OK: mid_rank ≥ last boundary → top band
            if idx < top_band_idx {
                let diff = value as f64 - trim_mean;
                trim_var_sum += diff * diff * count as f64;
                trim_var_count += count;
            }
            cum += count;
        }
        let trim_stdev = if trim_var_count > 1 {
            (trim_var_sum / trim_var_count as f64).sqrt()
        } else {
            0.0
        };

        println!(
            "{INDENT}{:<label_w$} {:>skip$}{GAP}{:>mean_w$} {unit}",
            "mean min-p99",
            "",
            fmt_commas_f64(conv_f(trim_mean), 0),
        );
        println!(
            "{INDENT}{:<label_w$} {:>skip$}{GAP}{:>mean_w$} {unit}",
            "stdev min-p99",
            "",
            fmt_commas_f64(conv_f(trim_stdev), 0),
        );
    }
    println!();
}

/// Format an integer with thousands separators, e.g.
/// `12345` → `"12,345"`.
pub fn fmt_commas(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Format a float with `decimals` fractional digits and thousands
/// separators on the integer part.
pub fn fmt_commas_f64(n: f64, decimals: usize) -> String {
    let s = format!("{n:.decimals$}");
    let (sign, body) = match s.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", s.as_str()),
    };
    let (int_part, frac_part) = match body.find('.') {
        Some(i) => (&body[..i], &body[i..]),
        None => (body, ""),
    };
    let int_commas = match int_part.parse::<u64>() {
        Ok(n) => fmt_commas(n),
        // Non-finite values format as "inf"/"NaN" — no integer
        // part to comma-group; pass the text through as-is.
        Err(_) => int_part.to_string(),
    };
    format!("{sign}{int_commas}{frac_part}")
}
