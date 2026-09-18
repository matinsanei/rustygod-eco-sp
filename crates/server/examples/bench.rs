//! Minimal gRPC load/spike bench over the live server.
//! Usage: bench [load|spike]  — prints ~10 summary lines, nothing else.

use rustygod_proto::{
    ai::{semantic_search_client::SemanticSearchClient, SearchProductsRequest},
    product::{product_service_client::ProductServiceClient, GetProductRequest, ListProductsRequest},
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Stats {
    lat_us: Vec<u64>,
    errors: u64,
}

async fn worker(
    id: usize,
    stop: Instant,
    stats: Arc<Mutex<Stats>>,
    mix: u64,
) {
    let mut products = ProductServiceClient::connect("http://127.0.0.1:50051")
        .await
        .unwrap();
    let mut search = SemanticSearchClient::connect("http://127.0.0.1:50051")
        .await
        .unwrap();
    let mut n = 0u64;
    while Instant::now() < stop {
        n += 1;
        let t = Instant::now();
        let r = if (id as u64 + n) % 10 < 7 {
            products
                .list_products(ListProductsRequest {
                    channel: "default-channel".into(),
                    first: 10,
                    after: String::new(),
                    category_id: String::new(),
                })
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        } else if (id as u64 + n) % 10 < 9 {
            search
                .search_products(SearchProductsRequest {
                    query: "shirt".into(),
                    channel: "default-channel".into(),
                    first: 5,
                })
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        } else {
            products
                .get_product(GetProductRequest { id: "126".into() })
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        };
        let dt = t.elapsed().as_micros() as u64;
        let _ = mix;
        let mut s = stats.lock().unwrap();
        s.lat_us.push(dt);
        if r.is_err() {
            s.errors += 1;
        }
    }
}

fn report(name: &str, stats: &Stats, secs: f64, rss_before: u64, rss_after: u64) {
    let mut v = stats.lat_us.clone();
    v.sort_unstable();
    let n = v.len() as f64;
    let pct = |p: f64| v[((n * p) as usize).min(v.len().saturating_sub(1))] as f64 / 1000.0;
    let avg = v.iter().sum::<u64>() as f64 / n.max(1.0) / 1000.0;
    println!("[{name}] reqs={} rps={:.0} avg={:.1}ms p50={:.1}ms p99={:.1}ms max={:.1}ms errors={} rss={rss_before}MB->{rss_after}MB",
        v.len(), n / secs, avg, pct(0.5), pct(0.99), *v.last().unwrap_or(&0) as f64 / 1000.0, stats.errors);
}

fn server_rss_mb() -> u64 {
    let pid = std::process::Command::new("pgrep")
        .args(["-f", "rustygod-server"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
        .unwrap_or_default();
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        })
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

async fn phase(name: &str, conc: usize, secs: u64) {
    let stats = Arc::new(Mutex::new(Stats { lat_us: Vec::with_capacity( conc * secs as usize * 60), errors: 0 }));
    let rss_before = server_rss_mb();
    let stop = Instant::now() + Duration::from_secs(secs);
    let mut handles = Vec::new();
    for id in 0..conc {
        handles.push(tokio::spawn(worker(id, stop, stats.clone(), 0)));
    }
    let t = Instant::now();
    for h in handles {
        h.await.unwrap();
    }
    let el = t.elapsed().as_secs_f64();
    let s = stats.lock().unwrap();
    report(name, &s, el, rss_before, server_rss_mb());
}

#[tokio::main]
async fn main() {
    let mode = std::env::args().nth(1).unwrap_or("load".into());
    let conc: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let secs: u64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    println!("server_rss_idle={}MB", server_rss_mb());
    if mode == "spike" {
        let (c0, s0) = (20, 10);
        let (c1, s1) = (if conc > 0 { conc } else { 200 }, if secs > 0 { secs } else { 10 });
        phase("base ", c0, s0).await;
        phase("spike", c1, s1).await;
        phase("cool ", c0, s0).await;
    } else {
        phase("load ", if conc > 0 { conc } else { 50 }, if secs > 0 { secs } else { 30 }).await;
    }
    println!("done");
}
