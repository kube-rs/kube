---
apiVersion: admissionregistration.k8s.io/v1beta1
kind: MutatingWebhookConfiguration
metadata:
  name: admission-controller-demo
webhooks:
  - name: foo-admission.default.svc
    # Optionally restrict events from namespaces with a specific label.
    # namespaceSelector:
    #   matchLabels:
    #     some-label: "true"
    clientConfig:
      caBundle: "${CA_PEM_B64}"
      url: "https://${PRIVATE_IP}:8443/mutate"
      # For controllers behind k8s services, use the format below instead of a url
      #service:
      #  name: foo-admission
      #  namespace: default
      #  path: "/mutate"
    rules:
      - operations: ["CREATE", "UPDATE"]
        apiGroups: ["clux.dev"]
        apiVersions: ["v1"]
        resources: ["foos"]
    failurePolicy: Fail
    admissionReviewVersions: ["v1", "v1beta1"]
    sideEffects: None
    timeoutSeconds: 5
