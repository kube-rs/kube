use darling::FromMeta;
use serde::Deserialize;

/// This struct mirrors the fields of `k8s_openapi::CustomResourceColumnDefinition` to support
/// parsing from the `#[kube]` attribute.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintColumn {
    pub description: Option<String>,
    pub format: Option<String>,
    pub json_path: String,
    pub name: String,
    pub priority: Option<i32>,
    pub type_: String,
}

// The reasoning for this custom FromMeta implementation is parallel to the one for the
// scale subresource. The two reasons are:
//
// - For backwards-compatibility by keeping the option to supply a JSON string.
// - To be able to declare the printcolumn as a list of typed fields.
impl FromMeta for PrintColumn {
    /// This is implemented for backwards-compatibility. It allows that the printcolumn can be
    /// deserialized from a JSON string.
    fn from_string(value: &str) -> darling::Result<Self> {
        serde_json::from_str(value).map_err(darling::Error::custom)
    }

    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        let mut errors = darling::Error::accumulator();

        let mut description: (bool, Option<Option<String>>) = (false, None);
        let mut format: (bool, Option<Option<String>>) = (false, None);
        let mut json_path: (bool, Option<String>) = (false, None);
        let mut column_name: (bool, Option<String>) = (false, None);
        let mut priority: (bool, Option<Option<i32>>) = (false, None);
        let mut type_: (bool, Option<String>) = (false, None);

        for item in items {
            match item {
                darling::ast::NestedMeta::Meta(meta) => {
                    let name = darling::util::path_to_string(meta.path());

                    match name.as_str() {
                        "description" => {
                            if !description.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                description = (true, Some(path))
                            } else {
                                errors.push(darling::Error::duplicate_field("description").with_span(&meta));
                            }
                        }
                        "format" => {
                            if !format.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                format = (true, Some(path))
                            } else {
                                errors.push(darling::Error::duplicate_field("format").with_span(&meta));
                            }
                        }
                        "json_path" => {
                            if !json_path.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                json_path = (true, path)
                            } else {
                                errors.push(darling::Error::duplicate_field("json_path").with_span(&meta));
                            }
                        }
                        "name" => {
                            if !column_name.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                column_name = (true, path)
                            } else {
                                errors.push(darling::Error::duplicate_field("name").with_span(&meta));
                            }
                        }
                        "priority" => {
                            if !priority.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                priority = (true, Some(path))
                            } else {
                                errors.push(darling::Error::duplicate_field("priority").with_span(&meta));
                            }
                        }
                        "type_" => {
                            if !type_.0 {
                                let path = errors.handle(darling::FromMeta::from_meta(meta));
                                type_ = (true, path)
                            } else {
                                errors.push(darling::Error::duplicate_field("type_").with_span(&meta));
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

        if !json_path.0 && json_path.1.is_none() {
            errors.push(darling::Error::missing_field("json_path"));
        }

        if !column_name.0 && column_name.1.is_none() {
            errors.push(darling::Error::missing_field("name"));
        }

        if !type_.0 && type_.1.is_none() {
            errors.push(darling::Error::missing_field("type"));
        }

        errors.finish()?;

        Ok(Self {
            description: description.1.unwrap_or_default(),
            format: format.1.unwrap_or_default(),
            json_path: json_path.1.unwrap(),
            name: column_name.1.unwrap(),
            priority: priority.1.unwrap_or_default(),
            type_: type_.1.unwrap(),
        })
    }
}
