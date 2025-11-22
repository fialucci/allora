use crate::error::{Error, Result};
use crate::spec::{HttpOutboundAdapterSpecYamlParser, HttpOutboundAdaptersSpec};
use serde_yaml::Value as YamlValue;

pub struct HttpOutboundAdaptersSpecYamlParser;

impl HttpOutboundAdaptersSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<HttpOutboundAdaptersSpec> {
        let version_val = yaml
            .get("version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| Error::serialization("missing or invalid version"))?
            as u32;
        if version_val != 1 {
            return Err(Error::serialization(format!(
                "unsupported version {} (expected 1)",
                version_val
            )));
        }
        let root = yaml
            .get("http-outbound-adapters")
            .ok_or_else(|| Error::serialization("missing 'http-outbound-adapters'"))?;
        if !root.is_sequence() {
            return Err(Error::serialization(
                "'http-outbound-adapters' must be a sequence",
            ));
        }
        let seq = root.as_sequence().unwrap();
        if seq.is_empty() {
            return Err(Error::serialization(
                "'http-outbound-adapters' sequence must not be empty (minItems=1)",
            ));
        }
        let mut spec = HttpOutboundAdaptersSpec::new(version_val);
        for item in seq {
            if !item.is_mapping() {
                return Err(Error::serialization(
                    "http-outbound-adapter entry must be a mapping",
                ));
            }
            let mut mapping = serde_yaml::Mapping::new();
            mapping.insert(
                YamlValue::String("version".into()),
                YamlValue::Number(serde_yaml::Number::from(version_val)),
            );
            mapping.insert(
                YamlValue::String("http-outbound-adapter".into()),
                item.clone(),
            );
            let synthesized = YamlValue::Mapping(mapping);
            let single = HttpOutboundAdapterSpecYamlParser::parse_value(&synthesized)?;
            spec.push(single);
        }
        Ok(spec)
    }
    pub fn parse_str(raw: &str) -> Result<HttpOutboundAdaptersSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
