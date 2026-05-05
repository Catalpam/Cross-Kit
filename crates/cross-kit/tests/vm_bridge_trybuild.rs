#![cfg(not(coverage))]

#[test]
fn vm_bridge_compile_time_contracts() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/vm_bridge_infers_state.rs");
    t.pass("tests/ui/vm_bridge_infers_diff_list.rs");
    t.pass("tests/ui/vm_bridge_factory_defaults_bridge.rs");
    t.pass("tests/ui/vm_bridge_infers_std_arc_observer.rs");
    t.pass("tests/ui/vm_bridge_infers_qualified_paths.rs");
    t.pass("tests/ui/vm_bridge_path_factory.rs");
    t.pass("tests/ui/vm_bridge_path_diff_list.rs");
    t.pass("tests/ui/vm_bridge_path_legacy_same.rs");
    t.pass("tests/ui/vm_bridge_path_bridge_override.rs");
    t.pass("tests/ui/vm_bridge_path_qualified_diff_item.rs");
    t.pass("tests/ui/metadata_main_generates_binary.rs");
    t.pass("tests/ui/metadata_main_qualified_paths.rs");
    t.pass("tests/ui/metadata_main_absolute_paths.rs");
    t.compile_fail("tests/ui/vm_bridge_missing_state_getter.rs");
    t.compile_fail("tests/ui/vm_bridge_state_getter_returns_unit.rs");
    t.compile_fail("tests/ui/vm_bridge_state_getter_has_args.rs");
    t.compile_fail("tests/ui/vm_bridge_state_type_override_requires_getter.rs");
    t.compile_fail("tests/ui/vm_bridge_bad_subscribe_observer.rs");
    t.compile_fail("tests/ui/vm_bridge_bad_subscribe_arg_name.rs");
    t.compile_fail("tests/ui/vm_bridge_unsupported_observer_pointer.rs");
    t.compile_fail("tests/ui/vm_bridge_diff_list_missing_types.rs");
    t.compile_fail("tests/ui/vm_bridge_diff_list_missing_item_type.rs");
    t.compile_fail("tests/ui/vm_bridge_partial_factory.rs");
    t.compile_fail("tests/ui/vm_bridge_partial_factory_method.rs");
    t.compile_fail("tests/ui/vm_bridge_path_factory_type_conflict.rs");
    t.compile_fail("tests/ui/vm_bridge_path_factory_method_conflict.rs");
    t.compile_fail("tests/ui/vm_bridge_path_diff_conflict.rs");
    t.compile_fail("tests/ui/vm_bridge_path_item_conflict.rs");
    t.compile_fail("tests/ui/vm_bridge_path_factory_one_segment.rs");
    t.compile_fail("tests/ui/vm_bridge_path_factory_nested.rs");
    t.compile_fail("tests/ui/metadata_main_empty.rs");
    t.compile_fail("tests/ui/metadata_main_missing_trait.rs");
}
