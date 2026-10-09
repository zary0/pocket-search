// SPDX-License-Identifier: GPL-3.0-only
//! Host-only timings and requested allocation counts, not device or RSS metrics.
use chromatic_client::{Button, Packet};
use pocket_search::{Document, SearchIndex};
use serde_json::json;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    mem::size_of,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    time::Instant,
};

struct Meter;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static REQUESTED: AtomicUsize = AtomicUsize::new(0);

fn allocated(size: usize) {
    let live = LIVE.fetch_add(size, Relaxed) + size;
    if ACTIVE.load(Relaxed) {
        CALLS.fetch_add(1, Relaxed);
        REQUESTED.fetch_add(size, Relaxed);
        PEAK.fetch_max(live, Relaxed);
    }
}

// SAFETY: every allocation operation delegates to System with the original
// pointer/layout. Counters do not allocate, alter pointers, or change ownership.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size(), Relaxed);
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            LIVE.fetch_sub(layout.size(), Relaxed);
            allocated(size);
        }
        result
    }
}

#[global_allocator]
static ALLOCATOR: Meter = Meter;

fn allocations<T>(f: impl FnOnce() -> T) -> (T, serde_json::Value) {
    let baseline = LIVE.load(Relaxed);
    CALLS.store(0, Relaxed);
    REQUESTED.store(0, Relaxed);
    PEAK.store(baseline, Relaxed);
    ACTIVE.store(true, Relaxed);
    let result = f();
    ACTIVE.store(false, Relaxed);
    // Snapshot before constructing the JSON report, which allocates itself.
    let calls = CALLS.load(Relaxed);
    let requested = REQUESTED.load(Relaxed);
    let peak = PEAK.load(Relaxed).saturating_sub(baseline);
    let live = LIVE.load(Relaxed).saturating_sub(baseline);
    let report = json!({
        "allocation_calls": calls,
        "requested_bytes_total": requested,
        "peak_live_bytes_above_start": peak,
        "live_bytes_delta_at_return": live,
    });
    (result, report)
}

fn timing(iterations: usize, mut f: impl FnMut()) -> serde_json::Value {
    for _ in 0..10 {
        f();
    }
    let mut ns = [0.0; 5];
    for sample in &mut ns {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        *sample = start.elapsed().as_nanos() as f64 / iterations as f64;
    }
    ns.sort_by(f64::total_cmp);
    json!({"iterations_per_sample":iterations,"samples":5,
        "median_ns_per_operation":ns[2],"min_ns":ns[0],"max_ns":ns[4]})
}

fn documents() -> Vec<Document> {
    (0..3000)
        .map(|id| Document {
            url: format!("https://example.invalid/page/{id:05}"),
            title: format!("Chromatic reading guide {id}"),
            body: format!(
                "{} {}",
                "Chromatic handheld reader text with independent search and pictures. ".repeat(4),
                if id % 17 == 0 {
                    "network 日本語"
                } else {
                    "reading 本"
                },
            ),
            image_urls: vec![],
        })
        .collect()
}

fn main() {
    let corpus = documents();
    let (index, index_memory) = allocations(|| SearchIndex::new(corpus));
    let mut searches = Vec::new();
    for query in ["chromatic", "chromatic network", "日本語"] {
        let (hits, memory) = allocations(|| index.search(query, 10));
        let expected_ids: Vec<_> = hits.iter().map(|hit| hit.document_id).collect();
        let timing = timing(20, || {
            black_box(index.search(black_box(query), 10));
        });
        searches
            .push(json!({"query":query,"top_ids":expected_ids,"memory":memory,"timing":timing}));
    }
    let packet = index.chromatic_result("chromatic", 0).unwrap();
    let ((), decoder_memory) = allocations(|| {
        let view = Packet::decode(black_box(&packet)).unwrap();
        black_box(view.action(Button::A));
    });
    assert_eq!(decoder_memory["allocation_calls"], 0);
    let decoder_timing = timing(100_000, || {
        let view = Packet::decode(black_box(&packet)).unwrap();
        black_box(view.action(Button::A));
    });
    let jp_packet = index.chromatic_result("日本語", 0).unwrap();
    let jp_timing = timing(100_000, || {
        black_box(Packet::decode(black_box(&jp_packet)).unwrap());
    });
    let output = json!({
        "schema_version":1,
        "measurement":"host_only_not_chromatic",
        "host_arch":std::env::consts::ARCH,
        "host_os":std::env::consts::OS,
        "profile":"Cargo bench; record compiler and flags separately",
        "allocation_scope":"requested System allocator bytes; not RSS or TLS/display memory",
        "corpus":{"documents":3000,"source":"original deterministic fixtures, not web pages"},
        "index_construction_memory":index_memory,
        "search":searches,
        "decoder":{"ascii_packet_bytes":packet.len(),"memory":decoder_memory,"timing":decoder_timing,
            "japanese_timing":jp_timing,"host_packet_view_bytes":size_of::<Packet<'static>>()},
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
