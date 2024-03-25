//! Simple example that demonstrates how to use raw API Server requests.
//! Raw requests are the equivalent of `kubectl get --raw`, and enable
//! users to query for objects or data that is not available through the
//! clientsets (i.e. Api<K>, whether primitive or derived from a CRD).
//! The example builds a tool similar to `kubectl top nodes`.
//! By default, node summary metrics data is fetched by Kubernetes from the
//! kubelet. The kubelet itself supports statistics access through CRI, or
//! through cAdvisor.
use k8s_openapi::{api::core::v1::Node, apimachinery::pkg::api::resource::Quantity};
use kube::{api::ListParams, Api, ResourceExt};
use serde::Deserialize;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = kube::Client::try_default().await?;

    let api: Api<Node> = Api::all(client.clone());
    let nodes = api.list(&ListParams::default()).await?;
    let mut summaries = Vec::new();

    for node in nodes {
        let name = node.name_any();

        // Query node stats by issuing a request to the admin endpoint.
        // See https://kubernetes.io/docs/reference/instrumentation/node-metrics/
        let url = format!("/api/v1/nodes/{}/proxy/stats/summary", name);
        let req = http::Request::get(url).body(Default::default())?;

        // Deserialize JSON response as a JSON value. Alternatively, a type that
        // implements `Deserialize` can be used.
        let resp = client.request::<serde_json::Value>(req).await?;

        // Our JSON value is an object so we can treat it like a dictionary.
        let summary = resp
            .get("node")
            .expect("node summary should exist in kubelet's admin endpoint");

        // The base JSON representation includes a lot of metrics, including
        // container metrics. Use a `NodeMetrics` type to deserialize only the
        // values we care about.
        let metrics = serde_json::from_value::<NodeMetrics>(summary.to_owned())?;

        // Get the current allocatable values for the node we are looking at and
        // save in a table we will use to print the results.
        let allocatable = node.status.unwrap_or_default().allocatable.unwrap_or_default();

        summaries.push(NodeSummary {
            name,
            metrics,
            allocatable,
        })
    }

    print_table(summaries);
    Ok(())
}

/// Contains a node's stats, its total allocatable memory and CPU and its CPU
/// and memory usage metrics.
#[derive(Debug)]
struct NodeSummary {
    name: String,
    metrics: NodeMetrics,
    allocatable: NodeAlloc,
}

/// Information on the CPU and memory usage of a node as returned by its
/// kubelet.
#[derive(Debug, Deserialize)]
struct NodeMetrics {
    cpu: Metric,
    memory: Metric,
}

// Convenience alias
type NodeAlloc = std::collections::BTreeMap<String, Quantity>;

/// A metric is either the CPU usage (represented as a share of the CPU's whole
/// core value) or the memory usage (represented in bytes)
/// None of these metrics are cumulative.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Metric {
    #[serde(rename_all = "camelCase")]
    Cpu { usage_nano_cores: usize },

    #[serde(rename_all = "camelCase")]
    Memory { usage_bytes: usize },
}

fn print_table(summaries: Vec<NodeSummary>) {
    use headers::*;

    // Each column (except for name) should be as wide as the length of its
    // header plus some additional slack to make it look prettier.
    let w_used_mem = USED_MEM.len() + 4;
    let w_used_cpu = USED_CPU.len() + 2;
    let w_percent_mem = PERCENT_MEM.len() + 2;
    let w_percent_cpu = PERCENT_CPU.len() + 4;

    // Width of name column should accommodate the longest node name present in
    // the list of summaries
    let w_name = {
        let max_name_width = summaries
            .iter()
            .map(|summary| summary.name.len())
            .max()
            .unwrap_or_else(|| 0)
            .max(NAME.len());
        max_name_width + 4
    };

    println!(
            "{NAME:w_name$} {USED_MEM:w_used_mem$} {PERCENT_MEM:w_percent_mem$} {USED_CPU:w_used_cpu$} {PERCENT_CPU:w_percent_cpu$}"
        );
    for summary in summaries {
        // Get Node memory allocatable and trim measurement suffix.
        let mem_total = summary
            .allocatable
            .get("memory")
            .map(|mem| {
                let mem = mem.0.trim_end_matches("Ki");
                mem.parse::<usize>().ok().unwrap_or_else(|| 1)
            })
            .unwrap_or_else(|| 1);

        // CPU allocatable quantity on the node does not have a measurement,
        // but is assumed to be whole cores.
        let cpu_total = summary
            .allocatable
            .get("cpu")
            .map(|mem| mem.0.parse::<usize>().ok().unwrap_or_else(|| 1))
            .unwrap_or_else(|| 1);

        let name = summary.name;
        let (percent_mem, used_mem) = summary.metrics.memory.convert_to_stat(mem_total);
        let (percent_cpu, used_cpu) = summary.metrics.cpu.convert_to_stat(cpu_total);

        println!("{name:w_name$} {used_mem:<w_used_mem$} {percent_mem:<w_percent_mem$} {used_cpu:<w_used_cpu$} {percent_cpu:<w_percent_cpu$}");
    }
}

// === impl Metric ===

impl Metric {
    // Convert measurement to what we will use in the table.
    // - CPU values are represented in millicores
    // - Memory values are represented in MiB (mebibyte)
    fn convert_to_stat(&self, alloc_total: usize) -> (String, String) {
        match self {
            // 1 millicore = 1000th of a CPU, 1 nano core = 1 billionth of a CPU
            // convert nano to milli
            Metric::Cpu { usage_nano_cores } => {
                // 1 millicore is a 1000th of a CPU. Our values are in
                // nanocores (a billionth of a CPU), so convert from nano to
                // milli.
                let cpu_m = (usage_nano_cores / (1000 * 1000)) as f64;
                // Convert a whole core to a millicore value
                let alloc_m = (alloc_total * 1000) as f64;
                // Calculate percentage
                let used = (cpu_m / alloc_m * 100.0) as usize;

                (format!("{used}%"), format!("{}m", cpu_m as usize))
            }

            Metric::Memory { usage_bytes } => {
                // 1 MiB = 2^20 bytes
                let mem_mib = *usage_bytes as f64 / (u64::pow(2, 20)) as f64;
                // 1 MiB = 2^10 KiB
                let alloc_mib = alloc_total as f64 / (u64::pow(2, 10)) as f64;
                let used = ((mem_mib / alloc_mib) * 100.0) as usize;
                (format!("{used}%"), format!("{}Mi", mem_mib as usize))
            }
        }
    }
}

/// Namespaces a group of constants used as the stat table headers.
// This way, the names do not have to be prefixed with `HEADER_`.
pub mod headers {
    pub const NAME: &str = "NAME";
    pub const USED_MEM: &str = "MEMORY(bytes)";
    pub const USED_CPU: &str = "CPU(cores)";
    pub const PERCENT_MEM: &str = "MEMORY%";
    pub const PERCENT_CPU: &str = "CPU%";
}
