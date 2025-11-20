use allora_runtime::spec::HttpInboundAdapterSpecYamlParser;

fn parse_err(yaml: &str, needle: &str) {
    let err = HttpInboundAdapterSpecYamlParser::parse_str(yaml).expect_err("expected error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains(needle), "msg='{}' needle='{}'", msg, needle)
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapter_yaml_invalid_yaml_error() {
    let err = HttpInboundAdapterSpecYamlParser::parse_str("::not yaml")
        .expect_err("invalid YAML should error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("yaml parse error")),
        _ => panic!("wrong error variant"),
    }
}

#[test]
fn http_inbound_adapter_yaml_missing_version_error() {
    parse_err("http-inbound-adapter:\n  host: 1\n", "missing 'version'");
}

#[test]
fn http_inbound_adapter_yaml_version_non_integer_error() {
    parse_err(
        "version: '1'\nhttp-inbound-adapter:\n  host: h\n",
        "'version' must be integer",
    );
}

#[test]
fn http_inbound_adapter_yaml_version_unsupported_error() {
    parse_err(
        "version: 2\nhttp-inbound-adapter:\n  host: h\n",
        "unsupported version",
    );
}

#[test]
fn http_inbound_adapter_yaml_root_not_mapping_error() {
    parse_err(
        "version: 1\nhttp-inbound-adapter: 42\n",
        "must be a mapping",
    );
}

#[test]
fn http_inbound_adapter_yaml_missing_host_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  port: 1\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "host required");
}

#[test]
fn http_inbound_adapter_yaml_host_wrong_type_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: 123\n  port: 1\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "host must be string");
}

#[test]
fn http_inbound_adapter_yaml_host_empty_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: ''\n  port: 1\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "host must not be empty");
}

#[test]
fn http_inbound_adapter_yaml_missing_port_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "port required");
}

#[test]
fn http_inbound_adapter_yaml_port_wrong_type_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 'abc'\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "port must be integer");
}

#[test]
fn http_inbound_adapter_yaml_port_zero_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 0\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "port out of range");
}

#[test]
fn http_inbound_adapter_yaml_port_out_of_range_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 65536\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "port out of range");
}

#[test]
fn http_inbound_adapter_yaml_missing_path_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  methods: [GET]\n  request-channel: ch\n", "path required");
}

#[test]
fn http_inbound_adapter_yaml_path_wrong_type_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: 123\n  methods: [GET]\n  request-channel: ch\n", "path must be string");
}

#[test]
fn http_inbound_adapter_yaml_path_empty_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: ''\n  methods: [GET]\n  request-channel: ch\n", "path must not be empty");
}

#[test]
fn http_inbound_adapter_yaml_path_missing_slash_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: p\n  methods: [GET]\n  request-channel: ch\n", "path must start with '/'");
}

#[test]
fn http_inbound_adapter_yaml_missing_methods_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  request-channel: ch\n", "methods required");
}

#[test]
fn http_inbound_adapter_yaml_methods_wrong_type_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: GET\n  request-channel: ch\n", "methods must be a sequence");
}

#[test]
fn http_inbound_adapter_yaml_methods_empty_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: []\n  request-channel: ch\n", "methods sequence must not be empty");
}

#[test]
fn http_inbound_adapter_yaml_method_not_string_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: [123]\n  request-channel: ch\n", "method must be string");
}

#[test]
fn http_inbound_adapter_yaml_method_unsupported_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: [FOO]\n  request-channel: ch\n", "unsupported http-inbound-adapter.method");
}

#[test]
fn http_inbound_adapter_yaml_missing_request_channel_error() {
    parse_err(
        "version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: [GET]\n",
        "request-channel required",
    );
}

#[test]
fn http_inbound_adapter_yaml_request_channel_wrong_type_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: [GET]\n  request-channel: []\n", "request-channel must be string");
}

#[test]
fn http_inbound_adapter_yaml_request_channel_empty_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: [GET]\n  request-channel: ''\n", "request-channel must not be empty");
}

#[test]
fn http_inbound_adapter_yaml_reply_channel_empty_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  host: h\n  port: 1\n  path: /p\n  methods: [GET]\n  request-channel: ch\n  reply-channel: ''\n", "reply-channel must not be empty");
}

#[test]
fn http_inbound_adapter_yaml_id_empty_error() {
    parse_err("version: 1\nhttp-inbound-adapter:\n  id: ''\n  host: h\n  port: 1\n  path: /p\n  methods: [GET]\n  request-channel: ch\n", "id must not be empty");
}
