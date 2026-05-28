//! YAML parser for [`crate::spec::AggregatorsSpec`] (collection v1).
//!
//! Delegates per-entry parsing to [`crate::spec::AggregatorSpecYamlParser`]
//! and focuses on sequence + version validation. Mirrors
//! [`crate::spec::FiltersSpecYamlParser`].
//!
//! # Accepted Shape (Informal)
//! ```yaml
//! version: 1
//! aggregators:
//!   - id: finality_quorum
//!     correlation_header: block_hash
//!     completion: chain.validator_quorum
//!     strategy: allora.emit_signal
//!     store: chain.persistent_finality
//!   - correlation_header: oracle_submission_id
//!     completion: chain.oracle_data_quorum
//! ```
//!
//! Uniqueness + auto-id generation are deferred to
//! [`crate::dsl::build_aggregators_from_spec`].

use crate::error::{Error, Result};
use crate::spec::aggregators_spec::AggregatorsSpec;
use crate::spec::version::validate_version;
use crate::spec::AggregatorSpecYamlParser;
use serde_yaml::Value as YamlValue;

pub struct AggregatorsSpecYamlParser;

impl AggregatorsSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<AggregatorsSpec> {
        let v = validate_version(yaml)?;
        let aggs_val = yaml
            .get("aggregators")
            .ok_or_else(|| Error::serialization("missing 'aggregators'"))?;
        if !aggs_val.is_sequence() {
            return Err(Error::serialization("'aggregators' must be a sequence"));
        }
        let seq = aggs_val.as_sequence().unwrap();
        if seq.is_empty() {
            return Err(Error::serialization(
                "'aggregators' sequence must not be empty (minItems=1)",
            ));
        }
        let mut spec = AggregatorsSpec::new(v);
        for item in seq {
            if !item.is_mapping() {
                return Err(Error::serialization("aggregator entry must be a mapping"));
            }
            // Synthesize a single-aggregator doc so the existing parser is reused.
            let mut obj = serde_yaml::Mapping::new();
            obj.insert(
                YamlValue::String("version".into()),
                YamlValue::Number(serde_yaml::Number::from(v)),
            );
            obj.insert(YamlValue::String("aggregator".into()), item.clone());
            let synthesized = YamlValue::Mapping(obj);
            spec.push(AggregatorSpecYamlParser::parse_value(&synthesized)?);
        }
        Ok(spec)
    }

    pub fn parse_str(raw: &str) -> Result<AggregatorsSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
