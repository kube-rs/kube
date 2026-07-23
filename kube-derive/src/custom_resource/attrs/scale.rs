use darling::FromMeta;
use serde::Deserialize;

/// This struct mirrors the fields of `k8s_openapi::CustomResourceSubresourceScale` to support
/// parsing from the `#[kube]` attribute.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scale {
    pub label_selector_path: Option<String>,
    pub spec_replicas_path: String,
    pub status_replicas_path: String,
}

// This custom FromMeta implementation is needed for two reasons:
//
// - To enable backwards-compatibility. Up to version 0.97.0 it was only possible to set scale
//   subresource values as a JSON string.
// - To be able to declare the scale sub-resource as a list of typed fields. The from_list impl uses
//   the derived implementation as inspiration.
impl FromMeta for Scale {
    /// This is implemented for backwards-compatibility. It allows that the scale subresource can
    /// be deserialized from a JSON string.
    fn from_string(value: &str) -> darling::Result<Self> {
        serde_json::from_str(value).map_err(darling::Error::custom)
    }

    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        let mut errors = darling::Error::accumulator();

        let mut label_selector_path: (bool, Option<Option<String>>) = (false, None);
        let mut spec_replicas_path: (bool, Option<String>) = (false, None);
        let mut status_replicas_path: (bool, Option<String>) = (false, None);

        for item in items {
            match item {
                darling::ast::NestedMeta::Meta(meta) => {
                    let name = darling::util::path_to_string(meta.path());

                    match name.as_str() {
                        "label_selector_path" => {
                            if !label_selector_path.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                label_selector_path = (true, Some(path))
                            } else {
                                errors.push(
                                    darling::Error::duplicate_field("label_selector_path").with_span(&meta),
                                );
                            }
                        }
                        "spec_replicas_path" => {
                            if !spec_replicas_path.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                spec_replicas_path = (true, path)
                            } else {
                                errors.push(
                                    darling::Error::duplicate_field("spec_replicas_path").with_span(&meta),
                                );
                            }
                        }
                        "status_replicas_path" => {
                            if !status_replicas_path.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                status_replicas_path = (true, path)
                            } else {
                                errors.push(
                                    darling::Error::duplicate_field("status_replicas_path").with_span(&meta),
                                );
                            }
                        }
                        other => errors.push(darling::Error::unknown_field(other)),
                    }
                }
                darling::ast::NestedMeta::Lit(lit) => {
                    errors.push(darling::Error::unsupported_format("literal").with_span(&lit.span()))
                }
            }
        }

        if !spec_replicas_path.0 && spec_replicas_path.1.is_none() {
            errors.push(darling::Error::missing_field("spec_replicas_path"));
        }

        if !status_replicas_path.0 && status_replicas_path.1.is_none() {
            errors.push(darling::Error::missing_field("status_replicas_path"));
        }

        errors.finish()?;

        Ok(Self {
            label_selector_path: label_selector_path.1.unwrap_or_default(),
            spec_replicas_path: spec_replicas_path.1.unwrap(),
            status_replicas_path: status_replicas_path.1.unwrap(),
        })
    }
}
