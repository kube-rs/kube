extern crate failure;
extern crate k8s_openapi;
extern crate kubernetes;

use k8s_openapi::v1_10::api::core::v1;
use kubernetes::client::APIClient;
use kubernetes::config;

fn main() {
    let kubeconfig = config::load_kube_config().expect("failed to load kubeconfig");
    let kubeclient = APIClient::new(kubeconfig);
    let req = v1::Pod::list_core_v1_namespaced_pod(
        "kube-system",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ).expect("failed to define list pod");
    let list_pod = kubeclient
        .request::<v1::PodList>(req)
        .expect("failed to list up pods");
    println!("{:?}", list_pod);
}
