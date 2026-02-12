//! Memory benchmark for kube-runtime watcher/reflector.
//!
//! Measures peak heap, total allocated bytes, and allocation count
//! across realistic watcher scenarios using dhat for heap profiling.
//!
//! Run with: `cargo bench -p kube-runtime --bench memory`

use std::collections::BTreeMap;

use futures::{StreamExt, stream};
use k8s_openapi::{
    api::core::v1::ConfigMap,
    apimachinery::pkg::apis::meta::v1::{ManagedFieldsEntry, ObjectMeta},
};
use kube_runtime::{
    reflector::{self, store},
    watcher,
};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

// ---------------------------------------------------------------------------
// ConfigMap generators
// ---------------------------------------------------------------------------

fn generate_configmaps(count: usize) -> Vec<ConfigMap> {
    (0..count)
        .map(|i| {
            let mut labels = BTreeMap::new();
            labels.insert("app".to_string(), "bench".to_string());
            labels.insert("instance".to_string(), format!("cm-{i}"));

            let mut annotations = BTreeMap::new();
            annotations.insert(
                "kubectl.kubernetes.io/last-applied-configuration".to_string(),
                format!("{{\"kind\":\"ConfigMap\",\"metadata\":{{\"name\":\"cm-{i}\"}},\"data\":{{}}}}"),
            );

            let mut data = BTreeMap::new();
            data.insert(
                "config.yaml".to_string(),
                format!("setting: value-{i}\ncount: {i}"),
            );
            data.insert("extra".to_string(), "x".repeat(128));

            ConfigMap {
                metadata: ObjectMeta {
                    name: Some(format!("cm-{i}")),
                    namespace: Some("bench".to_string()),
                    resource_version: Some(format!("{}", 1000 + i)),
                    uid: Some(format!("uid-{i}")),
                    labels: Some(labels),
                    annotations: Some(annotations),
                    ..ObjectMeta::default()
                },
                data: Some(data),
                ..ConfigMap::default()
            }
        })
        .collect()
}

fn generate_configmaps_with_managed_fields(count: usize) -> Vec<ConfigMap> {
    let mut cms = generate_configmaps(count);
    for cm in &mut cms {
        let managed_fields = vec![
            ManagedFieldsEntry {
                manager: Some("kubectl-client-side-apply".to_string()),
                operation: Some("Apply".to_string()),
                api_version: Some("v1".to_string()),
                fields_type: Some("FieldsV1".to_string()),
                fields_v1: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::FieldsV1(
                    serde_json::json!({
                        "f:data": { "f:config.yaml": {}, "f:extra": {} },
                        "f:metadata": { "f:labels": { "f:app": {}, "f:instance": {} } }
                    }),
                )),
                ..ManagedFieldsEntry::default()
            },
            ManagedFieldsEntry {
                manager: Some("kube-controller-manager".to_string()),
                operation: Some("Update".to_string()),
                api_version: Some("v1".to_string()),
                fields_type: Some("FieldsV1".to_string()),
                fields_v1: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::FieldsV1(
                    serde_json::json!({
                        "f:metadata": { "f:annotations": {} }
                    }),
                )),
                ..ManagedFieldsEntry::default()
            },
        ];
        cm.metadata.managed_fields = Some(managed_fields);
    }
    cms
}

// ---------------------------------------------------------------------------
// Event stream builders
// ---------------------------------------------------------------------------

fn init_events(cms: &[ConfigMap]) -> Vec<watcher::Result<watcher::Event<ConfigMap>>> {
    let mut events = Vec::with_capacity(cms.len() + 2);
    events.push(Ok(watcher::Event::Init));
    for cm in cms {
        events.push(Ok(watcher::Event::InitApply(cm.clone())));
    }
    events.push(Ok(watcher::Event::InitDone));
    events
}

fn steady_state_events(
    cms: &[ConfigMap],
    update_count: usize,
) -> Vec<watcher::Result<watcher::Event<ConfigMap>>> {
    let mut events = init_events(cms);

    // Apply updates to existing objects (cycle through them)
    for i in 0..update_count {
        let idx = i % cms.len();
        let mut cm = cms[idx].clone();
        cm.metadata.resource_version = Some(format!("{}", 100_000 + i));
        events.push(Ok(watcher::Event::Apply(cm)));
    }

    // Delete some objects
    let delete_count = update_count / 5;
    for i in 0..delete_count {
        let idx = (cms.len() / 2 + i) % cms.len();
        events.push(Ok(watcher::Event::Delete(cms[idx].clone())));
    }

    events
}

fn relist_events(cms: &[ConfigMap], update_count: usize) -> Vec<watcher::Result<watcher::Event<ConfigMap>>> {
    // First full list
    let mut events = steady_state_events(cms, update_count);

    // Second re-list (simulates desync recovery)
    events.push(Ok(watcher::Event::Init));
    for cm in cms {
        events.push(Ok(watcher::Event::InitApply(cm.clone())));
    }
    events.push(Ok(watcher::Event::InitDone));
    events
}

// ---------------------------------------------------------------------------
// Benchmark runner
// ---------------------------------------------------------------------------

async fn run_reflector(events: Vec<watcher::Result<watcher::Event<ConfigMap>>>) -> store::Store<ConfigMap> {
    let store_w = store::Writer::default();
    let store = store_w.as_reader();
    reflector::reflector(store_w, stream::iter(events))
        .map(|_| ())
        .collect::<()>()
        .await;
    store
}

async fn run_reflector_with_modify(
    events: Vec<watcher::Result<watcher::Event<ConfigMap>>>,
) -> store::Store<ConfigMap> {
    let store_w = store::Writer::default();
    let store = store_w.as_reader();
    reflector::reflector(
        store_w,
        stream::iter(events.into_iter().map(|ev| {
            ev.map(|e| {
                e.modify(|cm| {
                    cm.metadata.managed_fields = None;
                })
            })
        })),
    )
    .map(|_| ())
    .collect::<()>()
    .await;
    store
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

const NUM_OBJECTS: usize = 10_000;
const NUM_UPDATES: usize = 5_000;

/// Single benchmark metric in github-action-benchmark's `customSmallerIsBetter` format.
#[derive(serde::Serialize)]
struct BenchMetric {
    name: String,
    unit: &'static str,
    value: u64,
}

fn collect_stats(scenario: &str, results: &mut Vec<BenchMetric>) {
    let stats = dhat::HeapStats::get();
    results.push(BenchMetric {
        name: format!("{scenario} - peak_bytes"),
        unit: "bytes",
        value: stats.max_bytes as u64,
    });
    results.push(BenchMetric {
        name: format!("{scenario} - total_allocated"),
        unit: "bytes",
        value: stats.total_bytes as u64,
    });
    results.push(BenchMetric {
        name: format!("{scenario} - alloc_count"),
        unit: "allocations",
        value: stats.total_blocks as u64,
    });
}

async fn bench_init_listwatch(results: &mut Vec<BenchMetric>) {
    let _profiler = dhat::Profiler::builder().testing().build();

    let cms = generate_configmaps(NUM_OBJECTS);
    let events = init_events(&cms);
    let store = run_reflector(events).await;

    assert_eq!(
        store.state().len(),
        NUM_OBJECTS,
        "store should contain all objects"
    );
    collect_stats("init_listwatch", results);
}

async fn bench_steady_state(results: &mut Vec<BenchMetric>) {
    let _profiler = dhat::Profiler::builder().testing().build();

    let cms = generate_configmaps(NUM_OBJECTS);
    let events = steady_state_events(&cms, NUM_UPDATES);
    let store = run_reflector(events).await;

    let delete_count = NUM_UPDATES / 5;
    let expected_min = NUM_OBJECTS.saturating_sub(delete_count);
    assert!(
        store.state().len() >= expected_min,
        "store should contain at least {expected_min} objects, got {}",
        store.state().len()
    );
    collect_stats("steady_state", results);
}

async fn bench_relist(results: &mut Vec<BenchMetric>) {
    let _profiler = dhat::Profiler::builder().testing().build();

    let cms = generate_configmaps(NUM_OBJECTS);
    let events = relist_events(&cms, NUM_UPDATES);
    let store = run_reflector(events).await;

    assert_eq!(
        store.state().len(),
        NUM_OBJECTS,
        "store should contain all objects after relist"
    );
    collect_stats("relist", results);
}

async fn bench_init_without_modify(results: &mut Vec<BenchMetric>) {
    let _profiler = dhat::Profiler::builder().testing().build();

    let cms = generate_configmaps_with_managed_fields(NUM_OBJECTS);
    let events = init_events(&cms);
    let store = run_reflector(events).await;

    assert_eq!(
        store.state().len(),
        NUM_OBJECTS,
        "store should contain all objects without modify"
    );
    collect_stats("init_without_modify", results);
}

async fn bench_init_with_modify(results: &mut Vec<BenchMetric>) {
    let _profiler = dhat::Profiler::builder().testing().build();

    let cms = generate_configmaps_with_managed_fields(NUM_OBJECTS);
    let events = init_events(&cms);
    let store = run_reflector_with_modify(events).await;

    assert_eq!(
        store.state().len(),
        NUM_OBJECTS,
        "store should contain all objects after modify"
    );

    // Verify managed_fields were actually stripped
    for obj in store.state() {
        assert!(
            obj.metadata.managed_fields.is_none() || obj.metadata.managed_fields.as_ref().unwrap().is_empty(),
            "managed_fields should be stripped by modify()"
        );
    }
    collect_stats("init_with_modify", results);
}

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    let mut results = Vec::new();
    rt.block_on(async {
        bench_init_listwatch(&mut results).await;
        bench_steady_state(&mut results).await;
        bench_relist(&mut results).await;
        bench_init_without_modify(&mut results).await;
        bench_init_with_modify(&mut results).await;
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&results).expect("failed to serialize results")
    );
}
