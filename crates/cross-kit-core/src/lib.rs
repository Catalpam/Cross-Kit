//! Shared core constants and models for Cross-Kit crates.

use serde::{Deserialize, Serialize};

/// Default Cross-Kit project configuration file name.
pub const CONFIG_FILE_NAME: &str = "cross-kit.toml";

/// Current VM metadata schema version emitted by Cross-Kit macros.
pub const VM_METADATA_SCHEMA_VERSION: u32 = 1;

/// Target-independent VM metadata consumed by platform code generators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmMetadata {
    pub schema_version: u32,
    pub rust_type: String,
    pub bridge_name: String,
    pub mode: VmMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observer: Option<ObserverMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_item_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub factory: Option<FactoryMetadata>,
    pub methods: Vec<MethodMetadata>,
}

impl VmMetadata {
    pub fn validate(&self) -> Result<(), MetadataValidationError> {
        if self.schema_version != VM_METADATA_SCHEMA_VERSION {
            return Err(MetadataValidationError::UnsupportedSchemaVersion {
                actual: self.schema_version,
                expected: VM_METADATA_SCHEMA_VERSION,
            });
        }
        require_non_empty("rust_type", &self.rust_type)?;
        require_non_empty("bridge_name", &self.bridge_name)?;
        if let Some(factory) = &self.factory {
            require_non_empty("factory.rust_type", &factory.rust_type)?;
            require_non_empty("factory.method", &factory.method)?;
            require_non_empty("factory.bridge_name", &factory.bridge_name)?;
        }
        for method in &self.methods {
            require_non_empty("methods.name", &method.name)?;
            require_non_empty("methods.return_type", &method.return_type)?;
            for arg in &method.args {
                require_non_empty("methods.args.name", &arg.name)?;
                require_non_empty("methods.args.rust_type", &arg.rust_type)?;
            }
        }

        match self.mode {
            VmMode::State => {
                self.require_observer()?;
                require_optional("state_type", self.state_type.as_deref())?;
                self.require_observer_subscription()?;
                self.require_state_getter()?;
            }
            VmMode::DiffList => {
                self.require_observer()?;
                require_optional("diff_type", self.diff_type.as_deref())?;
                require_optional("list_item_type", self.list_item_type.as_deref())?;
                self.require_observer_subscription()?;
            }
            VmMode::Event => {
                self.require_observer()?;
                self.require_observer_subscription()?;
            }
            VmMode::Unknown => {
                return Err(MetadataValidationError::UnknownMode);
            }
        }

        Ok(())
    }

    fn require_observer(&self) -> Result<(), MetadataValidationError> {
        let observer = self
            .observer
            .as_ref()
            .ok_or(MetadataValidationError::MissingField("observer"))?;
        require_non_empty("observer.rust_type", &observer.rust_type)?;
        require_non_empty("observer.method", &observer.method)
    }

    fn require_method(&self, method_name: &'static str) -> Result<(), MetadataValidationError> {
        if self.methods.iter().any(|method| method.name == method_name) {
            Ok(())
        } else {
            Err(MetadataValidationError::MissingMethod(method_name))
        }
    }

    fn require_observer_subscription(&self) -> Result<(), MetadataValidationError> {
        self.require_method("subscribe")
    }

    fn require_state_getter(&self) -> Result<(), MetadataValidationError> {
        let method = self
            .methods
            .iter()
            .find(|method| method.name == "get_state")
            .ok_or(MetadataValidationError::MissingMethod("get_state"))?;
        if !method.args.is_empty() {
            return Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "must not accept arguments",
            });
        }
        if method.return_type != self.state_type.as_deref().unwrap_or_default() {
            return Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "return type must match state_type",
            });
        }
        Ok(())
    }
}

/// VM change notification style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmMode {
    State,
    DiffList,
    Event,
    #[serde(other)]
    Unknown,
}

/// Observer callback contract for a VM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverMetadata {
    pub rust_type: String,
    pub method: String,
}

/// Parent factory used to create a child VM bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryMetadata {
    pub rust_type: String,
    pub method: String,
    pub bridge_name: String,
}

/// Public VM method exposed to platform bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodMetadata {
    pub name: String,
    #[serde(default)]
    pub args: Vec<ArgMetadata>,
    pub return_type: String,
}

/// Public VM method argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgMetadata {
    pub name: String,
    pub rust_type: String,
}

/// Validation failure for target-independent metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataValidationError {
    EmptyField(&'static str),
    MissingField(&'static str),
    MissingMethod(&'static str),
    InvalidMethodShape {
        method: &'static str,
        reason: &'static str,
    },
    UnknownMode,
    UnsupportedSchemaVersion {
        actual: u32,
        expected: u32,
    },
}

impl std::fmt::Display for MetadataValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "metadata field `{field}` must not be empty"),
            Self::MissingField(field) => write!(f, "metadata field `{field}` is required"),
            Self::MissingMethod(method) => write!(f, "metadata method `{method}` is required"),
            Self::InvalidMethodShape { method, reason } => {
                write!(f, "metadata method `{method}` is invalid: {reason}")
            }
            Self::UnknownMode => write!(f, "metadata mode is unknown"),
            Self::UnsupportedSchemaVersion { actual, expected } => write!(
                f,
                "metadata schema version {actual} is unsupported; expected {expected}"
            ),
        }
    }
}

impl std::error::Error for MetadataValidationError {}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), MetadataValidationError> {
    if value.trim().is_empty() {
        Err(MetadataValidationError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn require_optional(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), MetadataValidationError> {
    let value = value.ok_or(MetadataValidationError::MissingField(field))?;
    require_non_empty(field, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_name_matches_cli_contract() {
        assert_eq!(CONFIG_FILE_NAME, "cross-kit.toml");
    }

    #[test]
    fn parses_and_validates_state_metadata() {
        let metadata: VmMetadata = serde_json::from_value(serde_json::json!({
            "schema_version": VM_METADATA_SCHEMA_VERSION,
            "rust_type": "CounterViewModel",
            "bridge_name": "CounterViewModelBridge",
            "mode": "state",
            "observer": {
                "rust_type": "CounterObserver",
                "method": "on_state"
            },
            "state_type": "CounterState",
            "factory": {
                "rust_type": "AppViewModel",
                "method": "make_counter_vm",
                "bridge_name": "AppViewModelBridge"
            },
            "methods": [
                {
                    "name": "get_state",
                    "args": [],
                    "return_type": "CounterState"
                },
                {
                    "name": "subscribe",
                    "args": [{"name": "observer", "rust_type": "Arc<dyn CounterObserver>"}],
                    "return_type": "i64"
                },
                {
                    "name": "increment_by",
                    "args": [{"name": "delta", "rust_type": "i32"}],
                    "return_type": "CounterState"
                }
            ]
        }))
        .unwrap();

        assert_eq!(metadata.mode, VmMode::State);
        assert_eq!(
            metadata.factory.as_ref().unwrap().bridge_name,
            "AppViewModelBridge"
        );
        metadata.validate().unwrap();
    }

    #[test]
    fn parses_diff_list_metadata_with_collection_types() {
        let metadata: VmMetadata = serde_json::from_value(serde_json::json!({
            "schema_version": VM_METADATA_SCHEMA_VERSION,
            "rust_type": "ListViewModel",
            "bridge_name": "ListViewModelBridge",
            "mode": "diff_list",
            "observer": {
                "rust_type": "ListObserver",
                "method": "on_diffs"
            },
            "diff_type": "ListDiff",
            "list_item_type": "ListItem",
            "methods": [
                {
                    "name": "subscribe",
                    "args": [{"name": "observer", "rust_type": "Arc<dyn ListObserver>"}],
                    "return_type": "i64"
                },
                {
                    "name": "apply_diffs",
                    "args": [{"name": "diffs", "rust_type": "Vec<ListDiff>"}],
                    "return_type": "bool"
                },
                {
                    "name": "find_item",
                    "args": [{"name": "id", "rust_type": "i64"}],
                    "return_type": "Option<ListItem>"
                }
            ]
        }))
        .unwrap();

        assert_eq!(metadata.mode, VmMode::DiffList);
        assert_eq!(
            metadata.methods[2].args[0].rust_type, "i64",
            "record and enum type names are passed through as Rust type strings"
        );
        metadata.validate().unwrap();
    }

    #[test]
    fn rejects_state_metadata_without_required_get_state_method() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn CounterObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };

        assert_eq!(
            metadata.validate(),
            Err(MetadataValidationError::MissingMethod("get_state"))
        );
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION + 1,
            rust_type: "EventViewModel".to_string(),
            bridge_name: "EventViewModelBridge".to_string(),
            mode: VmMode::Event,
            observer: Some(ObserverMetadata {
                rust_type: "EventObserver".to_string(),
                method: "on_event".to_string(),
            }),
            state_type: None,
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn EventObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };

        assert_eq!(
            metadata.validate(),
            Err(MetadataValidationError::UnsupportedSchemaVersion {
                actual: VM_METADATA_SCHEMA_VERSION + 1,
                expected: VM_METADATA_SCHEMA_VERSION
            })
        );
    }

    #[test]
    fn validates_event_metadata_with_observer() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "EventViewModel".to_string(),
            bridge_name: "EventViewModelBridge".to_string(),
            mode: VmMode::Event,
            observer: Some(ObserverMetadata {
                rust_type: "EventObserver".to_string(),
                method: "on_event".to_string(),
            }),
            state_type: None,
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn EventObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };

        metadata.validate().unwrap();
    }

    #[test]
    fn rejects_unknown_mode_and_missing_observer() {
        let unknown_mode = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "UnknownViewModel".to_string(),
            bridge_name: "UnknownViewModelBridge".to_string(),
            mode: VmMode::Unknown,
            observer: Some(ObserverMetadata {
                rust_type: "UnknownObserver".to_string(),
                method: "on_unknown".to_string(),
            }),
            state_type: None,
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: Vec::new(),
        };
        assert_eq!(
            unknown_mode.validate(),
            Err(MetadataValidationError::UnknownMode)
        );

        let missing_observer = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: None,
            state_type: None,
            diff_type: Some("ListDiff".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: Vec::new(),
        };
        assert_eq!(
            missing_observer.validate(),
            Err(MetadataValidationError::MissingField("observer"))
        );
    }

    #[test]
    fn rejects_empty_required_fields() {
        let empty_root_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: " ".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                MethodMetadata {
                    name: "get_state".to_string(),
                    args: Vec::new(),
                    return_type: "CounterState".to_string(),
                },
            ],
        };
        assert_eq!(
            empty_root_field.validate(),
            Err(MetadataValidationError::EmptyField("rust_type"))
        );

        let empty_optional_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: Some(ObserverMetadata {
                rust_type: "ListObserver".to_string(),
                method: "on_diffs".to_string(),
            }),
            state_type: None,
            diff_type: Some(" ".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: Vec::new(),
        };
        assert_eq!(
            empty_optional_field.validate(),
            Err(MetadataValidationError::EmptyField("diff_type"))
        );
    }

    #[test]
    fn rejects_empty_factory_and_method_fields() {
        let empty_factory_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: Some(FactoryMetadata {
                rust_type: "AppViewModel".to_string(),
                method: " ".to_string(),
                bridge_name: "AppViewModelBridge".to_string(),
            }),
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: "Arc<dyn CounterObserver>".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };
        assert_eq!(
            empty_factory_field.validate(),
            Err(MetadataValidationError::EmptyField("factory.method"))
        );

        let empty_method_field = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: Some(ObserverMetadata {
                rust_type: "ListObserver".to_string(),
                method: "on_diffs".to_string(),
            }),
            state_type: None,
            diff_type: Some("ListDiff".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: vec![MethodMetadata {
                name: "subscribe".to_string(),
                args: vec![ArgMetadata {
                    name: "observer".to_string(),
                    rust_type: " ".to_string(),
                }],
                return_type: "i64".to_string(),
            }],
        };
        assert_eq!(
            empty_method_field.validate(),
            Err(MetadataValidationError::EmptyField(
                "methods.args.rust_type"
            ))
        );
    }

    #[test]
    fn rejects_observer_vm_without_subscribe_method() {
        let metadata = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "ListViewModel".to_string(),
            bridge_name: "ListViewModelBridge".to_string(),
            mode: VmMode::DiffList,
            observer: Some(ObserverMetadata {
                rust_type: "ListObserver".to_string(),
                method: "on_diffs".to_string(),
            }),
            state_type: None,
            diff_type: Some("ListDiff".to_string()),
            list_item_type: Some("ListItem".to_string()),
            factory: None,
            methods: vec![MethodMetadata {
                name: "apply_diffs".to_string(),
                args: Vec::new(),
                return_type: "bool".to_string(),
            }],
        };

        assert_eq!(
            metadata.validate(),
            Err(MetadataValidationError::MissingMethod("subscribe"))
        );
    }

    #[test]
    fn rejects_state_getter_with_wrong_shape() {
        let wrong_args = VmMetadata {
            schema_version: VM_METADATA_SCHEMA_VERSION,
            rust_type: "CounterViewModel".to_string(),
            bridge_name: "CounterViewModelBridge".to_string(),
            mode: VmMode::State,
            observer: Some(ObserverMetadata {
                rust_type: "CounterObserver".to_string(),
                method: "on_state".to_string(),
            }),
            state_type: Some("CounterState".to_string()),
            diff_type: None,
            list_item_type: None,
            factory: None,
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                MethodMetadata {
                    name: "get_state".to_string(),
                    args: vec![ArgMetadata {
                        name: "id".to_string(),
                        rust_type: "i64".to_string(),
                    }],
                    return_type: "CounterState".to_string(),
                },
            ],
        };
        assert_eq!(
            wrong_args.validate(),
            Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "must not accept arguments"
            })
        );

        let wrong_return = VmMetadata {
            methods: vec![
                MethodMetadata {
                    name: "subscribe".to_string(),
                    args: vec![ArgMetadata {
                        name: "observer".to_string(),
                        rust_type: "Arc<dyn CounterObserver>".to_string(),
                    }],
                    return_type: "i64".to_string(),
                },
                MethodMetadata {
                    name: "get_state".to_string(),
                    args: Vec::new(),
                    return_type: "OtherState".to_string(),
                },
            ],
            ..wrong_args
        };
        assert_eq!(
            wrong_return.validate(),
            Err(MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "return type must match state_type"
            })
        );
    }

    #[test]
    fn validation_errors_have_actionable_messages() {
        assert_eq!(
            MetadataValidationError::EmptyField("rust_type").to_string(),
            "metadata field `rust_type` must not be empty"
        );
        assert_eq!(
            MetadataValidationError::MissingField("observer").to_string(),
            "metadata field `observer` is required"
        );
        assert_eq!(
            MetadataValidationError::MissingMethod("get_state").to_string(),
            "metadata method `get_state` is required"
        );
        assert_eq!(
            (MetadataValidationError::InvalidMethodShape {
                method: "get_state",
                reason: "must not accept arguments",
            })
            .to_string(),
            "metadata method `get_state` is invalid: must not accept arguments"
        );
        assert_eq!(
            MetadataValidationError::UnknownMode.to_string(),
            "metadata mode is unknown"
        );
        assert_eq!(
            (MetadataValidationError::UnsupportedSchemaVersion {
                actual: 2,
                expected: 1,
            })
            .to_string(),
            "metadata schema version 2 is unsupported; expected 1"
        );
    }

    #[test]
    fn fixture_ir_contracts_parse_and_validate() {
        let fixture = include_str!("../../../fixtures/metadata/counter-list.json");
        let metadatas: Vec<VmMetadata> = serde_json::from_str(fixture).unwrap();

        for metadata in metadatas {
            metadata.validate().unwrap();
        }
    }
}
