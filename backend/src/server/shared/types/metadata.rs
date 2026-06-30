use std::borrow::Cow;

use serde::Serialize;
use utoipa::ToSchema;

use super::{Color, Icon};

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct MetadataRegistry {
    pub service_definitions: Vec<TypeMetadata>,
    pub subnet_types: Vec<TypeMetadata>,
    pub edge_types: Vec<TypeMetadata>,
    pub group_types: Vec<TypeMetadata>,
    pub entities: Vec<EntityMetadata>,
    pub ports: Vec<TypeMetadata>,
    pub discovery_types: Vec<TypeMetadata>,
    pub billing_plans: Vec<TypeMetadata>,
    pub features: Vec<TypeMetadata>,
    pub permissions: Vec<TypeMetadata>,
    pub concepts: Vec<EntityMetadata>,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct TypeMetadata {
    pub id: &'static str,
    #[schema(required)]
    pub name: Option<&'static str>,
    /// Most providers supply a `&'static str` (wrapped as `Cow::Borrowed`); types
    /// whose description is composed at build time (e.g. credential transports
    /// derive theirs from a centralized integration description) supply
    /// `Cow::Owned`.
    #[schema(value_type = Option<String>, required)]
    pub description: Option<Cow<'static, str>>,
    #[schema(required)]
    pub category: Option<&'static str>,
    #[schema(value_type = Option<String>, required)]
    pub icon: Option<Icon>,
    pub color: Color,
    #[schema(required)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct EntityMetadata {
    pub id: &'static str,
    pub color: Color,
    #[schema(value_type = String)]
    pub icon: Icon,
}

pub trait HasId {
    fn id(&self) -> &'static str;
}

pub trait MetadataProvider<T>: HasId {
    fn to_metadata(&self) -> T;
}

pub trait EntityMetadataProvider: MetadataProvider<EntityMetadata> {
    fn color(&self) -> Color;
    fn icon(&self) -> Icon;
}

pub trait TypeMetadataProvider: EntityMetadataProvider + MetadataProvider<TypeMetadata> {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str {
        ""
    }
    fn category(&self) -> &'static str {
        ""
    }
    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

impl<T> MetadataProvider<EntityMetadata> for T
where
    T: EntityMetadataProvider,
{
    fn to_metadata(&self) -> EntityMetadata {
        EntityMetadata {
            id: self.id(),
            color: self.color(),
            icon: self.icon(),
        }
    }
}

impl<T> MetadataProvider<TypeMetadata> for T
where
    T: TypeMetadataProvider,
{
    fn to_metadata(&self) -> TypeMetadata {
        let id = self.id();
        let name = self.name();
        let description = self.description();
        let category = self.category();
        let icon = self.icon();
        let color = self.color();
        let metadata = self.metadata();

        TypeMetadata {
            id,
            name: (!name.is_empty()).then_some(name),
            description: (!description.is_empty()).then_some(Cow::Borrowed(description)),
            category: (!category.is_empty()).then_some(category),
            icon: Some(icon),
            color,
            metadata: (!metadata.as_object().is_some_and(|obj| obj.is_empty())).then_some(metadata),
        }
    }
}
