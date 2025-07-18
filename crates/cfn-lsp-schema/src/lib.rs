use std::{collections::HashMap, io::Write as _, sync::OnceLock};

use proc_macro2::{Ident, Punct, Spacing, Span, TokenStream};
use quote::TokenStreamExt;
use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum SchemaError {
    #[error("extracting resource info from file {filename}")]
    ExtractingResourceInfo {
        filename: String,
        source: Box<SchemaError>,
    },
    #[error("reading file contents")]
    ReadFile(#[from] std::io::Error),
    #[error("parsing json from file {filename}: {json_error}")]
    ParseJson {
        filename: String,
        json_error: serde_json::Error,
    },
    #[error("zip archive error")]
    ZipError(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, SchemaError>;

pub fn render_to<P>(output_path: P) -> Result<()>
where
    P: AsRef<std::path::Path>,
{
    let input_path = concat!(env!("CARGO_MANIFEST_DIR"), "/CloudformationSchema.zip");
    let f = std::fs::File::open(input_path)?;
    let resources = extract_from_bundle(f)?;

    let mut resource_entry_tokens = Vec::new();
    for resource in resources {
        let type_name = resource.type_name;

        let description = match resource.description {
            Some(d) => quote::quote! { Some(#d.to_string()) },
            None => quote::quote! { None },
        };

        let mut permissions_statements = Vec::new();
        for (handler, permissions) in resource.handler_permissions {
            if let Some(permissions) = permissions {
                let append_statements: Vec<_> = permissions
                    .iter()
                    .map(|s| {
                        quote::quote! {
                            handler_permissions.push(#s.to_string());
                        }
                    })
                    .collect();
                if !append_statements.is_empty() {
                    permissions_statements.push(quote::quote! {
                        {
                            let mut handler_permissions =Vec::new();

                            #(#append_statements)*

                            permissions.insert(#handler, handler_permissions);
                        }
                    });
                }
            }
        }

        let tokens = if !permissions_statements.is_empty() {
            quote::quote! {
                #type_name => {
                    let mut permissions = HashMap::new();

                    #(#permissions_statements)*

                    let info = ResourceInfo {
                        description: #description,
                        handler_permissions: permissions,
                    };
                    Some(info)
                },
            }
        } else {
            quote::quote! {
                #type_name => {
                    let info = ResourceInfo {
                        description: #description,
                        handler_permissions: HashMap::new(),
                    };
                    Some(info)
                },
            }
        };

        resource_entry_tokens.push(tokens);
    }

    let tokens = quote::quote! {
        use std::collections::HashMap;

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
        pub enum Handler {
            Create,
            Read,
            Update,
            Delete,
        }

        #[derive(Debug)]
        pub struct ResourceInfo {
            pub description: Option<String>,
            pub handler_permissions: HashMap<Handler, Vec<String>>,
        }

        pub fn info_for_resource(resource_type: &str) -> Option<ResourceInfo> {
            match resource_type {
                #(#resource_entry_tokens)*
                other => None,
            }
        }
    };

    let mut output_file = std::fs::File::create(output_path)?;
    output_file.write_all(format!("{tokens}").as_bytes())?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Handler {
    Create,
    Read,
    Update,
    Delete,
}

impl std::fmt::Display for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Handler::Create => f.write_str("Create"),
            Handler::Read => f.write_str("Read"),
            Handler::Update => f.write_str("Update"),
            Handler::Delete => f.write_str("Delete"),
        }
    }
}

impl quote::ToTokens for Handler {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Handler", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Alone));
        match *self {
            Handler::Create => tokens.append(Ident::new("Create", Span::call_site())),
            Handler::Read => tokens.append(Ident::new("Read", Span::call_site())),
            Handler::Update => tokens.append(Ident::new("Update", Span::call_site())),
            Handler::Delete => tokens.append(Ident::new("Delete", Span::call_site())),
        }
    }
}

#[derive(Debug)]
pub struct ResourceInfo {
    pub type_name: String,
    pub description: Option<String>,
    pub handler_permissions: HashMap<Handler, Option<Vec<String>>>,
    pub create_only_properties: Vec<String>,
    pub primary_identifier: String,
    pub read_only_properties: Vec<String>,
    pub write_only_properties: Vec<String>,
}

#[derive(Deserialize)]
struct Schema {
    #[serde(rename = "typeName")]
    type_name: String,
    description: Option<String>,
    handlers: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    #[serde(rename = "createOnlyProperties")]
    create_only_properties: Option<Vec<String>>,
    #[serde(rename = "primaryIdentifier")]
    primary_identifier: Option<Vec<String>>,
    #[serde(rename = "readOnlyProperties")]
    read_only_properties: Option<Vec<String>>,
    #[serde(rename = "writeOnlyProperties")]
    write_only_properties: Option<Vec<String>>,
}

fn strip_properties_prefix(s: String) -> String {
    s.replace("/properties/", "")
}

fn extract_from_file<R>(filename: &str, reader: R) -> Result<ResourceInfo>
where
    R: std::io::Read,
{
    let schema: Schema =
        serde_json::from_reader(reader).map_err(|json_error| SchemaError::ParseJson {
            filename: filename.to_string(),
            json_error,
        })?;

    let mut resource_info = ResourceInfo {
        type_name: schema.type_name,
        description: schema.description,
        handler_permissions: HashMap::new(),
        read_only_properties: schema
            .read_only_properties
            .unwrap_or_default()
            .into_iter()
            .map(strip_properties_prefix)
            .collect(),
        write_only_properties: schema
            .write_only_properties
            .unwrap_or_default()
            .into_iter()
            .map(strip_properties_prefix)
            .collect(),
        create_only_properties: schema
            .create_only_properties
            .unwrap_or_default()
            .into_iter()
            .map(strip_properties_prefix)
            .collect(),
        primary_identifier: schema
            .primary_identifier
            .unwrap_or_default()
            .into_iter()
            .map(strip_properties_prefix)
            .collect::<Vec<_>>()
            .join("|"),
    };
    if let Some(handlers) = schema.handlers {
        for (handler, details) in handlers {
            let handler_type = match handler.as_str() {
                "create" => Handler::Create,
                "read" => Handler::Read,
                "update" => Handler::Update,
                "delete" => Handler::Delete,
                _ => continue, // Ignore unknown handlers
            };
            let permissions = details.get("permissions").cloned();
            if let Some(serde_json::Value::Array(permissions)) = permissions {
                let permissions = permissions
                    .into_iter()
                    .filter_map(|p| p.as_str().map(String::from))
                    .collect();
                resource_info
                    .handler_permissions
                    .insert(handler_type, Some(permissions));
            }
        }
    }
    Ok(resource_info)
}

#[cfg(not(feature = "bundle"))]
fn schema_archive() -> Result<zip::ZipArchive<std::fs::File>> {
    let input_path = concat!(env!("CARGO_MANIFEST_DIR"), "/CloudformationSchema.zip");
    let f = std::fs::File::open(input_path)?;
    let z = zip::ZipArchive::new(f).map_err(SchemaError::from)?;
    Ok(z)
}

#[cfg(feature = "bundle")]
fn schema_archive() -> Result<zip::ZipArchive<std::io::Cursor<&'static [u8]>>> {
    use std::io::Cursor;

    let contents = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/CloudformationSchema.zip"
    ));
    let z = zip::ZipArchive::new(Cursor::new(&contents[..])).map_err(SchemaError::from)?;
    Ok(z)
}

pub fn extract_resource_from_bundle(resource_type: &str) -> Result<ResourceInfo> {
    let mut z = schema_archive()?;
    let name = format!(
        "{}.json",
        resource_type.to_ascii_lowercase().replace("::", "-")
    );
    tracing::debug!(%name, %resource_type, transformed_name = resource_type.to_ascii_lowercase().replace("::", "-"), "looking for resource in zip archive");
    let zf = z.by_name(&name).map_err(SchemaError::ZipError)?;
    let resource_info =
        extract_from_file(&name, zf).map_err(|source| SchemaError::ExtractingResourceInfo {
            filename: name,
            source: Box::new(source),
        })?;
    Ok(resource_info)
}

fn extract_from_bundle<R>(reader: R) -> Result<Vec<ResourceInfo>>
where
    R: std::io::Read + std::io::Seek,
{
    let mut archive = zip::ZipArchive::new(reader).map_err(SchemaError::ZipError)?;
    let mut resources = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let filename = file.name().to_string();
        if filename.ends_with(".json") {
            let resource_info = extract_from_file(&filename, &mut file).map_err(|source| {
                SchemaError::ExtractingResourceInfo {
                    filename,
                    source: Box::new(source),
                }
            })?;
            resources.push(resource_info);
        }
    }
    Ok(resources)
}

pub struct Resource {
    pub type_name: String,
    pub description: Option<String>,
}

static RESOURCE_TYPES: OnceLock<Vec<Resource>> = OnceLock::new();

pub fn get_resource_types() -> &'static [Resource] {
    RESOURCE_TYPES.get_or_init(|| {
        let mut resources = Vec::new();

        let input_path = concat!(env!("CARGO_MANIFEST_DIR"), "/CloudformationSchema.zip");
        let f = std::fs::File::open(input_path).expect("Could not open schema zip file");
        let mut archive = zip::ZipArchive::new(f).expect("Could not read zip archive");
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("Could not read file from zip");
            let filename = file.name().to_string();
            if filename.ends_with(".json") {
                let schema: Schema =
                    serde_json::from_reader(&mut file).expect("Could not parse JSON schema");
                resources.push(Resource {
                    type_name: schema.type_name,
                    description: schema.description,
                });
            }
        }

        resources
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracting_from_file() {
        let filename = "testdata/aws-iam-role.json";
        let f = std::fs::File::open(filename).unwrap();
        let result = extract_from_file("aws-iam-role.json", f).unwrap();
        assert_eq!(result.type_name, "AWS::IAM::Role");
        assert_eq!(
            result
                .handler_permissions
                .get(&Handler::Create)
                .unwrap()
                .clone(),
            Some(vec![
                "iam:CreateRole".to_string(),
                "iam:PutRolePolicy".to_string(),
                "iam:AttachRolePolicy".to_string(),
                "iam:GetRolePolicy".to_string(),
                "iam:TagRole".to_string(),
                "iam:UntagRole".to_string(),
                "iam:GetRole".to_string(),
            ])
        );
    }
    #[test]
    fn extracting_from_bundle() {
        let filename = "CloudformationSchema.zip";
        let f = std::fs::File::open(filename).unwrap();
        let result = extract_from_bundle(f).unwrap();
        assert!(!result.is_empty());
        assert!(result.iter().any(|r| r.type_name == "AWS::IAM::Role"));
        assert!(result.iter().any(|r| r.type_name == "AWS::S3::Bucket"));
    }
}
