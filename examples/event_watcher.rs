use futures::{pin_mut, TryStreamExt};
use k8s_openapi::{
    api::{core::v1::ObjectReference, events::v1::Event},
    apimachinery::pkg::apis::meta::v1::Time,
    chrono::Utc,
};
use kube::{
    runtime::{watcher, WatchStreamExt},
    Api, Client, ResourceExt,
};

/// limited variant of `kubectl events` that works on current context's namespace
///
/// requires a new enough cluster that apis/events.k8s.io/v1 work (kubectl uses corev1::Event)
/// for old style usage of core::v1::Event see node_watcher
#[derive(clap::Parser)]
struct App {
    /// Filter by object and kind
    ///
    /// Using --for=Pod/blog-xxxxx
    /// Note that kind name is case sensitive
    #[arg(long)]
    r#for: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let app: App = clap::Parser::parse();

    let events: Api<Event> = Api::default_namespaced(client);
    let mut conf = watcher::Config::default();
    if let Some(forval) = app.r#for {
        if let Some((kind, name)) = forval.split_once('/') {
            conf = conf.fields(&format!("regarding.kind={kind},regarding.name={name}"));
        }
    }
    let event_stream = watcher(events, conf).default_backoff().applied_objects();
    pin_mut!(event_stream);

    println!("{0:<6} {1:<15} {2:<55} {3}", "AGE", "REASON", "OBJECT", "MESSAGE");
    while let Some(ev) = event_stream.try_next().await? {
        let age = ev.creation_timestamp().map(format_creation).unwrap_or_default();
        let reason = ev.reason.unwrap_or_default();
        let obj = ev.regarding.map(format_objref).flatten().unwrap_or_default();
        let note = ev.note.unwrap_or_default();
        println!("{0:<6} {1:<15} {2:<55} {3}", age, reason, obj, note);
    }
    Ok(())
}

fn format_objref(oref: ObjectReference) -> Option<String> {
    Some(format!("{}/{}", oref.kind?, oref.name?))
}

fn format_creation(time: Time) -> String {
    let dur = Utc::now().signed_duration_since(time.0);
    match (dur.num_days(), dur.num_hours(), dur.num_minutes()) {
        (days, _, _) if days > 0 => format!("{days}d"),
        (_, hours, _) if hours > 0 => format!("{hours}h"),
        (_, _, mins) => format!("{mins}m"),
    }
}
