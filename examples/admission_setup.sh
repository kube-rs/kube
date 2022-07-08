#!/usr/bin/env bash
set -euo pipefail

# This script is loosely adapting the TLS setup described in
# https://kubernetes.io/blog/2019/03/21/a-guide-to-kubernetes-admission-controllers/#tls-certificates
# for local development

# Require: a private ip reachable from your cluster.
# If using k3d to test locally, then probably 10.x.x.x or 192.168.X.X
# When running behind a Service in-cluster; 0.0.0.0
test -n "${ADMISSION_PRIVATE_IP}"

# Cleanup: Remove old MutatingWebhookConfiguration if exists (immutable)
kubectl delete mutatingwebhookconfiguration admission-controller-demo || true
# If behind a service:
#kubectl -n default delete secret admission-controller-tls || true

# Get your IP into the cert
echo "subjectAltName = IP:${ADMISSION_PRIVATE_IP}" > admission_extfile.cnf
# Or, if using DNS (e.g. when running behind a service):
#echo "subjectAltName = DNS:admission-controller.default.svc" > admission_extfile.cnf

# Generate the CA cert and private key
openssl req -nodes -new -x509 \
    -keyout ca.key \
    -out ca.crt -subj "/CN=admission-controller-demo"

# Generate the private key for the webhook server
openssl genrsa -out admission-controller-tls.key 2048

# Generate a Certificate Signing Request (CSR) for the private key
# and sign it with the private key of the CA.
openssl req -new -key admission-controller-tls.key \
    -subj "/CN=admission-controller-demo" \
    | openssl x509 -req -CA ca.crt -CAkey ca.key \
        -CAcreateserial -out admission-controller-tls.crt \
        -extfile admission_extfile.cnf

CA_PEM64="$(openssl base64 -A < ca.crt)"
# shellcheck disable=SC2016
sed -e 's@${CA_PEM_B64}@'"$CA_PEM64"'@g' < admission_controller.yaml.tpl |
    sed -e 's@${PRIVATE_IP}@'"$ADMISSION_PRIVATE_IP"'@g'  \
    | kubectl create -f -

# if behind a service:
#kubectl -n default create secret tls admission-controller-tls \
#    --cert admission-controller-tls.crt \
#    --key admission-controller-tls.key
# similar guide: https://www.openpolicyagent.org/docs/v0.11.0/kubernetes-admission-control/

# Sanity:
kubectl get mutatingwebhookconfiguration admission-controller-demo -oyaml
