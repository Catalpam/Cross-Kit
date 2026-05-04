# Cross-Kit VM Metadata Contract

This document defines the target-independent VM metadata contract emitted by
`cross_kit::vm_bridge` and consumed by Cross-Kit generators.

## Envelope

Each annotated VM emits one JSON object through `CkVmMetadata::ck_vm_metadata`.
The current envelope keeps legacy fields for the existing Swift packager while
introducing a stable `ir` object for new generators.

```json
{
  "schema_version": 1,
  "swift_bridge": "CounterViewModelBridge",
  "mode": "state",
  "vm_type": "CounterViewModel",
  "methods": [],
  "ir": {},
  "swift_code": "... deprecated compatibility field ..."
}
```

`swift_code` is not part of the long-term contract. It remains available only
until Swift generation is moved out of the macro and into a generator crate.
`fixtures/metadata/counter-list.json` snapshots the canonical `ir` array from
the shared example and is compared against generated metadata in tests.

## IR

`ir` is the canonical contract. It must not contain generated Swift, Kotlin, or
other target-language source.

```json
{
  "schema_version": 1,
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
      "name": "increment_by",
      "args": [{ "name": "delta", "rust_type": "i32" }],
      "return_type": "CounterState"
    }
  ]
}
```

Fields:

- `schema_version`: currently `1`. Generators must reject unsupported versions.
- `rust_type`: Rust VM type name.
- `bridge_name`: platform bridge type name requested by the VM annotation. New
  VMs should use `bridge = "..."`; `swift_bridge = "..."` remains accepted for
  existing examples during the Swift compatibility window.
- `mode`: one of `state`, `diff_list`, or `event`. `event` is reserved for the
  next contract expansion.
- `observer`: observer Rust type and callback method. Required for current VM
  modes.
- `state_type`: required for `state` VMs.
- `diff_type` and `list_item_type`: required for `diff_list` VMs.
- `factory`: optional parent VM factory used to construct child VMs.
- `methods`: public VM methods visible to generators.

Type strings are Rust type strings from the annotated impl. Generators are
responsible for mapping primitives, `Arc<T>`, `Option<T>`, `Vec<T>`, records,
and enums into each platform's native API.

## Validation

`cross-kit-core` owns the Rust model and validation rules:

- schema version must match `VM_METADATA_SCHEMA_VERSION`.
- `rust_type` and `bridge_name` must be non-empty.
- `state` requires `observer`, `state_type`, and `get_state`.
- `diff_list` requires `observer`, `diff_type`, and `list_item_type`.
- `event` currently requires `observer`.
- Observer modes require `subscribe` so generated bindings can attach callbacks.
- `state` requires `get_state` to accept no arguments and return `state_type`.

The compatibility envelope fields may change or disappear after generators stop
depending on macro-produced target code. `ir` is the stable input for new Swift
and Android codegen.
