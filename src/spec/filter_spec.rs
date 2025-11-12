//! FilterSpec: specification for a single message filter (EIP Message Filter).
//! Captures source channel id (`from`), optional target channel id (`to`), predicate expression (`when`),
//! and optional identifier (`id`).
//!
//! # Field Semantics
//! * `id` (optional): Explicit identifier for referencing / diagnostics. If absent, the multi-build
//!   collection builder (`build_filters_from_spec`) will generate a deterministic `filter:auto.N` id.
//!   Single filter builds (via `FilterSpecYamlParser` + `build_filter_from_spec`) do not auto-generate
//!   an id unless performed through the collection builder.
//! * `from` (required, non-empty): Channel id producing messages to be filtered.
//! * `to` (optional, non-empty if present): Downstream channel id for accepted messages (routing stage).
//! * `when` (required, non-empty): APL v1 expression evaluated against each `Exchange`.
//!
//! # Uniqueness & Validation
//! * YAML parser (`filter_spec_yaml.rs`) enforces presence/type and non-empty strings for `from`, `when`, `to`, `id` (if present).
//! * It deliberately does NOT enforce uniqueness across `id` values; duplicates are detected in the
//!   collection builder (`build_filters_from_spec`) which errors on \"duplicate filter.id\".
//!
//! # Runtime Mapping
//! * `FilterSpec` ids are not stored on the runtime `Filter` (pure predicate). The builder currently
//!   validates and/or generates ids for future diagnostics without attaching them to the runtime struct.
//! * Future enhancement may introduce a wrapper or runtime metadata layer to expose the originating id.
//!
//! # Construction Helpers
//! * `new(from, when)` – minimal spec without id or `to`.
//! * `with_to(from, to, when)` – adds `to`.
//! * `with_id(id, from, when)` – explicit id.
//! * `with_id_to(id, from, to, when)` – id + routing target.
//! * `set_id(id)` – internal use (builder assigning auto-generated id).
//! * `id()` – access the optional id.
//! * `from()` / `to()` / `when()` – access fields.
//! * `debug_dump()` – formatted spec summary for diagnostics.

#[derive(Debug, Clone)]
pub struct FilterSpec {
    id: Option<String>,
    from: String,
    to: Option<String>,
    when: String,
}

impl FilterSpec {
    pub fn new<F: Into<String>, W: Into<String>>(from: F, when: W) -> Self {
        Self {
            id: None,
            from: from.into(),
            to: None,
            when: when.into(),
        }
    }
    pub fn with_to<F: Into<String>, T: Into<String>, W: Into<String>>(
        from: F,
        to: T,
        when: W,
    ) -> Self {
        Self {
            id: None,
            from: from.into(),
            to: Some(to.into()),
            when: when.into(),
        }
    }
    pub fn with_id<F: Into<String>, W: Into<String>, I: Into<String>>(
        id: I,
        from: F,
        when: W,
    ) -> Self {
        Self {
            id: Some(id.into()),
            from: from.into(),
            to: None,
            when: when.into(),
        }
    }
    pub fn with_id_to<F: Into<String>, T: Into<String>, W: Into<String>, I: Into<String>>(
        id: I,
        from: F,
        to: T,
        when: W,
    ) -> Self {
        Self {
            id: Some(id.into()),
            from: from.into(),
            to: Some(to.into()),
            when: when.into(),
        }
    }
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    pub fn from(&self) -> &str {
        &self.from
    }
    pub fn to(&self) -> Option<&str> {
        self.to.as_deref()
    }
    pub fn when(&self) -> &str {
        &self.when
    }
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}
