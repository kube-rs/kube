#[macro_use] extern crate log;
use kube::{
    api::{ResourceType, Reflector},
    client::APIClient,
    config,
};
use k8s_openapi::api::core::v1::{PodSpec, PodStatus};

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let resource = ResourceType::Pods(Some("kube-system".into()));
    let rf : Reflector<PodSpec, PodStatus> = Reflector::new(client.clone(), resource.into())?;

    rf.read()?.into_iter().for_each(|(name, p)| {
        info!("Found pod {} ({}) with {:?}",
            name,
            p.status.phase.unwrap(),
            p.spec.containers.into_iter().map(|c| c.name).collect::<Vec<_>>(),
        );
    });

    // Here we both poll and reconcile based on events from the main thread
    // If you run this next to actix-web (say), spawn a thread and pass `rf` as app state
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        rf.poll()?;

        // Can also print internal state
        let pods = rf.read()?.into_iter().map(|(name, _)| name).collect::<Vec<_>>();
        info!("Current pods: {:?}", pods);
    }
}
