extern crate failure;
extern crate k8s_openapi;
extern crate kube;

use kube::{
    api::{ResourceType, Reflector},
    client::APIClient,
    config,
};

// You can fill in the parts of the structs you want
// but for full info, you probably want k8s_openapi
use k8s_openapi::api::apps::v1::{DeploymentSpec, DeploymentStatus};

fn main() -> Result<(), failure::Error> {
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let resource = ResourceType::Deploys(Some("kube-system".into()));
    let rf : Reflector<DeploymentSpec, DeploymentStatus> = Reflector::new(client, resource.into())?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> (DeploymentSpec, DeploymentStatus)
    rf.read()?.into_iter().for_each(|(name, d)| {
        println!("Found deployment for {} - {} replicas running {:?}",
            name, d.status.replicas.unwrap(),
            d.spec.template.spec.unwrap().containers
                .into_iter().map(|c| c.image.unwrap()).collect::<Vec<_>>()
        );
    });

    // r needs to have `r.poll()?` called continuosly to keep state up to date:
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        rf.poll()?;
        let deploys = rf.read()?.into_iter().map(|(name, _)| name).collect::<Vec<_>>();
        println!("Current deploys: {:?}", deploys);
    }
}
