use crate::error::{Error, Result};
use crate::spec::http_outbound_adapter_spec::{
    HttpOutboundAdapterSpec, HttpOutboundAdapterSpecRootV1,
};
use serde_yaml::{self, Value as YamlValue};

pub struct HttpOutboundAdapterSpecYamlParser;
impl HttpOutboundAdapterSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<HttpOutboundAdapterSpec> {
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
            .get("http-outbound-adapter")
            .ok_or_else(|| Error::serialization("missing 'http-outbound-adapter'"))?;
        if !root.is_mapping() {
            return Err(Error::serialization(
                "'http-outbound-adapter' must be a mapping",
            ));
        }
        let mut mapping = serde_yaml::Mapping::new();
        mapping.insert(
            YamlValue::String("version".into()),
            YamlValue::Number(serde_yaml::Number::from(version_val)),
        );
        mapping.insert(
            YamlValue::String("http-outbound-adapter".into()),
            root.clone(),
        );
        let synthesized = YamlValue::Mapping(mapping);
        let root: HttpOutboundAdapterSpecRootV1 = serde_yaml::from_value(synthesized)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        let blk = &root.http_outbound_adapter;
        if blk.url.is_empty() {
            return Err(Error::serialization(
                "http-outbound-adapter.url must not be empty",
            ));
        }
        // Validate URL eagerly so misconfiguration surfaces at config load,
        // not at first dispatch. Also enforce that a scheme is present and
        // is one of the schemes the underlying client supports.
        let parsed = url::Url::parse(&blk.url)
            .map_err(|e| Error::serialization(format!("http-outbound-adapter.url invalid: {e}")))?;
        match parsed.scheme() {
            "http" | "https" => {}
            other => {
                return Err(Error::serialization(format!(
                    "http-outbound-adapter.url has unsupported scheme '{}' (expected http or https)",
                    other
                )));
            }
        }
        if let Some(idv) = &blk.id {
            if idv.is_empty() {
                return Err(Error::serialization(
                    "http-outbound-adapter.id must not be empty",
                ));
            }
        }
        Ok(HttpOutboundAdapterSpec::from_block(
            root.http_outbound_adapter,
        ))
    }
    pub fn parse_str(raw: &str) -> Result<HttpOutboundAdapterSpec> {
        let yaml: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&yaml)
    }
}
