use std::{collections::HashMap, path::Path};

use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum SchemaError {
    #[error("reading file contents")]
    ReadFile(#[from] std::io::Error),
    #[error("parsing json")]
    ParseJson(#[from] serde_json::Error),
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
    handlers: Option<HashMap<String, HashMap<String, Vec<String>>>>,
}

fn extract_from_file(p: impl AsRef<Path>) -> Result<ResourceInfo> {
    let contents = std::fs::read_to_string(p).map_err(SchemaError::ReadFile)?;
    let schema: Schema = serde_json::from_str(&contents).map_err(SchemaError::ParseJson)?;

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
            resource_info
                .handler_permissions
                .insert(handler_type, permissions);
        }
    }
    Ok(resource_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracting_from_file() {
        let filename = "testdata/aws-iam-role.json";
        let result = extract_from_file(filename).unwrap();
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
}
