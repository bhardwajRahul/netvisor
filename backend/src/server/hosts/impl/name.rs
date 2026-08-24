//! Host naming: the name of a host, inseparable from the evidence that produced it.
//!
//! Before this module the question "did a person type this name, or did we derive it?" was
//! answered by inspecting the string — `name.parse::<IpAddr>().is_ok()`. That could recognise
//! exactly one derived shape, so a name derived from a detected service was indistinguishable
//! from a hand-typed one and froze forever, and a name supplied by a controller had nowhere to
//! sit in the ordering at all (GH #680).
//!
//! The first fix carried the rung in a second `HostBase` field beside `name`. Two fields that
//! must move together is a standing invitation to move only one: three construction sites
//! assigned `name` directly and let the rung default, and one of them shipped a host labelled
//! with an address it no longer held. Here the rung *is* the variant, so there is nothing to
//! forget.

use std::borrow::Cow;
use std::fmt::{self, Display};
use std::net::IpAddr;
use std::ops::Deref;

use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::{Display as StrumDisplay, EnumDiscriminants, EnumString, VariantNames};
use utoipa::{PartialSchema, ToSchema};

/// A host's name together with the rung of the naming ladder that produced it.
///
/// **Declaration order is the precedence order.** The generated [`HostNameSource`] discriminant
/// derives `Ord` from it, and that derive is the whole enforcement mechanism: a rung inserted at
/// its rank propagates to every comparison, with no per-call-site precedence to keep in sync.
///
/// Note the absence of `PartialOrd`/`Ord` on `HostName` itself — deriving them would compare the
/// variant *and then the payload*, so `Ip("10.0.0.2") < Ip("10.0.0.10")` and equal-rung refresh
/// would silently depend on string ordering. Compare [`HostName::source`], never the values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, EnumDiscriminants)]
#[strum_discriminants(name(HostNameSource))]
#[strum_discriminants(derive(
    PartialOrd,
    Ord,
    Hash,
    StrumDisplay,
    EnumString,
    VariantNames,
    Serialize,
    Deserialize,
    ToSchema
))]
pub enum HostName {
    /// No name at all.
    #[default]
    Unnamed,
    /// A name whose provenance we do not know: a payload from a daemon predating this release, or
    /// a row predating the column. Above nothing, below everything we can attribute.
    Unspecified(String),
    /// The host's own IP address, used because nothing better was known.
    Ip(String),
    /// The name of the best non-generic service detected on the host.
    DetectedService(String),
    /// Reverse DNS, a hostname the host reported, or SNMP sysName.
    Hostname(String),
    /// A name a person assigned in a controller (UniFi, HPE Instant On, …) and that the
    /// integration read back out. Deliberate, and stable across DHCP lease changes.
    Integration(String),
    /// A name a person typed into Scanopy. Nothing outranks it.
    Manual(String),
}

impl Default for HostNameSource {
    fn default() -> Self {
        Self::Unnamed
    }
}

impl HostName {
    /// The host's IP address — the bottom rung that still carries a name.
    pub fn from_ip(ip: IpAddr) -> Self {
        Self::Ip(ip.to_string())
    }

    /// Which rung produced this name.
    pub fn source(&self) -> HostNameSource {
        self.into()
    }

    /// The name itself. Empty for [`HostName::Unnamed`], the only variant carrying no value.
    pub fn value(&self) -> &str {
        match self {
            Self::Unnamed => "",
            Self::Unspecified(v)
            | Self::Ip(v)
            | Self::DetectedService(v)
            | Self::Hostname(v)
            | Self::Integration(v)
            | Self::Manual(v) => v,
        }
    }

    /// Whether this carries no usable name — `Unnamed`, or a value that is blank.
    pub fn is_blank(&self) -> bool {
        self.value().trim().is_empty()
    }

    /// Lower this name's rung to `ceiling` if it claims more, keeping the value.
    ///
    /// The server applies this to daemon payloads: `Manual` means "a person typed this into
    /// Scanopy", which nothing running on a daemon can know.
    pub fn clamped_to(self, ceiling: HostNameSource) -> Self {
        if self.source() <= ceiling {
            return self;
        }
        let value = self.value().to_string();
        Self::from_parts(value, ceiling)
    }

    /// Rebuild from a stored or received `(name, name_source)` pair. A blank value collapses to
    /// `Unnamed` whatever rung was claimed for it — a rung with no name means nothing.
    pub(crate) fn from_parts(value: String, source: HostNameSource) -> Self {
        if value.trim().is_empty() {
            return Self::Unnamed;
        }
        match source {
            HostNameSource::Unnamed => Self::Unnamed,
            HostNameSource::Unspecified => Self::Unspecified(value),
            HostNameSource::Ip => Self::Ip(value),
            HostNameSource::DetectedService => Self::DetectedService(value),
            HostNameSource::Hostname => Self::Hostname(value),
            HostNameSource::Integration => Self::Integration(value),
            HostNameSource::Manual => Self::Manual(value),
        }
    }
}

impl Deref for HostName {
    type Target = str;
    fn deref(&self) -> &str {
        self.value()
    }
}

impl AsRef<str> for HostName {
    fn as_ref(&self) -> &str {
        self.value()
    }
}

impl Display for HostName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value())
    }
}

impl PartialEq<str> for HostName {
    fn eq(&self, other: &str) -> bool {
        self.value() == other
    }
}

impl PartialEq<&str> for HostName {
    fn eq(&self, other: &&str) -> bool {
        self.value() == *other
    }
}

impl PartialEq<String> for HostName {
    fn eq(&self, other: &String) -> bool {
        self.value() == other.as_str()
    }
}

impl PartialEq<HostName> for str {
    fn eq(&self, other: &HostName) -> bool {
        self == other.value()
    }
}

impl PartialEq<HostName> for String {
    fn eq(&self, other: &HostName) -> bool {
        self.as_str() == other.value()
    }
}

/// Serialised as the two flat keys `name` and `name_source`, not as a tagged enum.
///
/// `name` has to stay a bare top-level string: daemons at 0.17.11 and earlier POST
/// `{"name": "<string>"}` with no rung at all, and the `name` column carries `ORDER BY` and the
/// free-text host search. Adjacent tagging would produce the right two keys but *requires* the
/// tag, so those payloads would fail to parse — hence the hand-rolled pair.
impl Serialize for HostName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("HostName", 2)?;
        state.serialize_field("name", self.value())?;
        state.serialize_field("name_source", &self.source())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for HostName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct HostNameVisitor;

        impl<'de> Visitor<'de> for HostNameVisitor {
            type Value = HostName;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object with a `name` and an optional `name_source`")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<HostName, A::Error> {
                let mut value: Option<String> = None;
                let mut source: Option<HostNameSource> = None;

                // Remaining keys belong to the flattening parent; skip rather than reject.
                while let Some(key) = map.next_key::<Cow<'_, str>>()? {
                    match key.as_ref() {
                        "name" => value = Some(map.next_value()?),
                        "name_source" => source = Some(map.next_value()?),
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                let value = value.unwrap_or_default();
                Ok(match source {
                    Some(source) => HostName::from_parts(value, source),
                    // A daemon predating the rung. Its name is real but unattributable, so it
                    // enters at the bottom and cannot displace anything we know the source of.
                    None if value.trim().is_empty() => HostName::Unnamed,
                    None => HostName::Unspecified(value),
                })
            }
        }

        deserializer.deserialize_map(HostNameVisitor)
    }
}

impl PartialSchema for HostName {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
        use utoipa::openapi::{RefOr, Schema};

        RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::new(Type::Object))
                .property(
                    "name",
                    ObjectBuilder::new()
                        .schema_type(SchemaType::new(Type::String))
                        .description(Some("Human-facing name for the host."))
                        .build(),
                )
                .required("name")
                // A `$ref` rather than `HostNameSource::schema()`, which would inline a second
                // copy of the enum here instead of pointing at the shared component.
                .property(
                    "name_source",
                    utoipa::openapi::Ref::from_schema_name("HostNameSource"),
                )
                .build(),
        ))
    }
}

impl ToSchema for HostName {
    fn name() -> Cow<'static, str> {
        Cow::Borrowed("HostName")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::name::HostName;

    #[test]
    fn a_payload_without_a_rung_is_unattributable_but_keeps_its_name() {
        // Every daemon released to date sends this shape. Losing the name would be a regression;
        // trusting it would let an old daemon overwrite a name a person typed.
        let parsed: HostName = serde_json::from_str(r#"{"name":"nas.lan"}"#).unwrap();
        assert_eq!(parsed, HostName::Unspecified("nas.lan".to_string()));
    }

    #[test]
    fn the_wire_shape_is_two_flat_keys() {
        let json = serde_json::to_value(HostName::Integration("Core Switch".to_string())).unwrap();
        assert_eq!(json["name"], "Core Switch");
        assert_eq!(json["name_source"], "Integration");
    }

    #[test]
    fn a_rung_without_a_name_collapses_to_unnamed() {
        let parsed: HostName =
            serde_json::from_str(r#"{"name":"","name_source":"Integration"}"#).unwrap();
        assert_eq!(parsed, HostName::Unnamed);
    }

    #[test]
    fn clamping_lowers_an_overreaching_claim_and_leaves_the_rest_alone() {
        let manual = HostName::Manual("typed".to_string());
        assert_eq!(
            manual.clamped_to(HostNameSource::Integration),
            HostName::Integration("typed".to_string())
        );

        let hostname = HostName::Hostname("nas.lan".to_string());
        assert_eq!(
            hostname.clone().clamped_to(HostNameSource::Integration),
            hostname
        );
    }
}
