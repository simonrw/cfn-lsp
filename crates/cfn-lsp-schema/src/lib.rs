use std::collections::HashMap;

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

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Handler {
    Create,
    Read,
    Update,
    Delete,
}
pub struct ResourceInfo {
    pub type_name: String,
    pub description: Option<String>,
    pub handler_permissions: HashMap<Handler, Option<Vec<String>>>,
}

#[derive(Deserialize)]
struct Schema {
    #[serde(rename = "typeName")]
    type_name: String,
    description: Option<String>,
    handlers: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
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

pub fn extract_from_bundle<R>(reader: R) -> Result<HashMap<String, ResourceInfo>>
where
    R: std::io::Read + std::io::Seek,
{
    let mut archive = zip::ZipArchive::new(reader).map_err(SchemaError::ZipError)?;
    let mut resources = HashMap::new();

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
            resources.insert(resource_info.type_name.clone(), resource_info);
        }
    }
    Ok(resources)
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
        assert!(result.contains_key("AWS::IAM::Role"));
        assert!(result.contains_key("AWS::S3::Bucket"));
    }
}
