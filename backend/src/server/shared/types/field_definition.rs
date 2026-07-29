use serde::Serialize;
use utoipa::ToSchema;

/// Field types for dynamic form generation
#[derive(Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Select,
}

/// Definition of a form field for dynamic UI rendering.
/// Used by fixture generation to produce JSON consumed by the frontend.
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct FieldDefinition {
    /// Server-assigned unique identifier.
    pub id: &'static str,
    /// Human-facing field label.
    pub label: &'static str,
    /// How the field should be rendered and validated.
    pub field_type: FieldType,
    /// Placeholder text for the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    /// Whether the value is a secret, so it is masked and never echoed back.
    pub secret: bool,
    /// Whether the field may be left empty.
    pub optional: bool,
    /// Explanatory text shown beneath the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<&'static str>,
    /// Choices for select-type fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<FieldOption>>,
    /// Value pre-filled when the field is first shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<&'static str>,
    /// Grouping label used to section a long form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'static str>,
}

/// An option for select-type fields
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct FieldOption {
    /// Human-facing option label.
    pub label: &'static str,
    /// Value submitted when this option is chosen.
    pub value: &'static str,
}
