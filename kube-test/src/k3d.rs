//! Test helper to create a temporary k3d cluster.
use std::{convert::TryFrom, process::Command};

use kube::{config::Kubeconfig, Client, Config};

/// Struct to manage a temporary k3d cluster.
#[derive(Debug, Default)]
pub struct TestEnv {
    // The name of the temporary cluster.
    name: String,
    // Kubeconfig of the temporary cluster.
    kubeconfig: Kubeconfig,
}

impl TestEnv {
    /// Builder for configuring the test environemnt.
    pub fn builder() -> TestEnvBuilder {
        Default::default()
    }

    /// Create the default minimal test environment.
    pub fn new() -> Self {
        Self::builder().build()
    }

    fn delete(&mut self) {
        tracing::info!("Deleting k3d cluster {}...", &self.name);
        let status = Command::new("k3d")
            .args(&["cluster", "delete", &self.name])
            .status()
            .expect("k3d cluster delete failed");
        assert!(
            status.success(),
            "k3d cluster delete failed. cluster {} may still exist",
            self.name
        );
    }

    /// Create a new `Client` configured for the temporary server.
    pub async fn client(&self) -> Client {
        assert_eq!(
            self.kubeconfig.clusters.len(),
            1,
            "kubeconfig only contains the temporary cluster"
        );
        assert_eq!(
            self.kubeconfig
                .clusters
                .get(0)
                .unwrap()
                .name
                .as_str()
                .strip_prefix("k3d-")
                .unwrap(),
            self.name,
            "kubeconfig only contains the temporary cluster"
        );

        let config = Config::from_custom_kubeconfig(self.kubeconfig.clone(), &Default::default())
            .await
            .expect("valid kubeconfig");
        Client::try_from(config).expect("client")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        self.delete();
    }
}

/// Builder for [`TestEnv`] to customize the environment.
#[derive(Debug)]
pub struct TestEnvBuilder {
    /// The number of servers. Default: 1
    servers: usize,
    /// The number of agents. Default: 0
    agents: usize,
    /// Inject the Host IP as `host.k3d.internal` into the containers and CoreDNS.
    /// Default: false
    host_ip_injection: bool,
    /// Create an image volume for importing images.
    /// Default: false
    create_image_volume: bool,
    /// Create a `LoadBalancer` in front of the server nodes.
    /// Default: false
    create_load_balancer: bool,
    /// Set `--verbose` flag. Default: false
    verbose: bool,
    /// Specify k3s version to use in format `v1.20.6`. Default: None.
    version: Option<String>,
}

impl Default for TestEnvBuilder {
    fn default() -> Self {
        Self {
            servers: 1,
            agents: 0,
            host_ip_injection: false,
            create_image_volume: false,
            create_load_balancer: false,
            verbose: false,
            version: None,
        }
    }
}

impl TestEnvBuilder {
    /// Set the number of servers in the temporary cluster.
    pub fn servers(&mut self, servers: usize) -> &mut Self {
        self.servers = servers;
        self
    }

    /// Set the number of agents in the temporary cluster.
    pub fn agents(&mut self, agents: usize) -> &mut Self {
        self.agents = agents;
        self
    }

    /// Enable host ip injection.
    pub fn inject_host_ip(&mut self) -> &mut Self {
        self.host_ip_injection = true;
        self
    }

    /// Create image volume.
    pub fn with_image_volume(&mut self) -> &mut Self {
        self.create_image_volume = true;
        self
    }

    /// Create load balancer.
    pub fn with_load_balancer(&mut self) -> &mut Self {
        self.create_load_balancer = true;
        self
    }

    /// Set `verbose` flag.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }

    /// Set the k3s version to use. The version should be in the form `v1.20.6`.
    pub fn k3s_version<T: Into<String>>(&mut self, version: T) -> &mut Self {
        self.version = Some(version.into());
        self
    }

    /// Create the test environment.
    pub fn build(&self) -> TestEnv {
        let name = xid::new().to_string();
        let servers = format!("--servers={}", self.servers);
        let agents = format!("--agents={}", self.agents);
        let mut args = vec![
            "cluster",
            "create",
            &name,
            "--wait",
            // Don't change `~/.kube/config`
            "--kubeconfig-update-default=false",
            "--kubeconfig-switch-context=false",
            // Disable to avoid having to create the default service account in each test.
            // > we're keeping this disabled because if enabled, default SA is
            // > missing which would force all tests to create one
            // > in normal apiserver operation this SA is created by controller, but that is
            // > not run in integration environment
            // > https://git.io/JZKFC
            "--k3s-server-arg",
            "--kube-apiserver-arg=disable-admission-plugins=ServiceAccount",
            // Disable components and features
            "--k3s-server-arg",
            "--disable=servicelb",
            "--k3s-server-arg",
            "--disable=traefik",
            "--k3s-server-arg",
            "--disable=metrics-server",
            "--k3s-server-arg",
            "--disable-cloud-controller",
            "--no-rollback",
            &servers,
            &agents,
        ];
        if self.verbose {
            args.push("--verbose");
        }
        if !self.host_ip_injection {
            args.push("--no-hostip");
        }
        if !self.create_image_volume {
            args.push("--no-image-volume");
        }
        if !self.create_load_balancer {
            args.push("--no-lb");
        }

        let mut args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        if let Some(version) = &self.version {
            args.push(format!("--image=rancher/k3s:{}-k3s1", version));
        }

        let status = Command::new("k3d")
            .args(&args)
            .status()
            .expect("k3d cluster create");
        assert!(status.success(), "failed to create k3d cluster");

        // Output the cluster's kubeconfig to stdout and store it.
        let stdout = Command::new("k3d")
            .args(&["kubeconfig", "get", &name])
            .output()
            .expect("k3d kubeconfig get failed")
            .stdout;
        let stdout = std::str::from_utf8(&stdout).expect("valid string");

        TestEnv {
            name,
            kubeconfig: serde_yaml::from_str(stdout).expect("valid kubeconfig"),
        }
    }
}
