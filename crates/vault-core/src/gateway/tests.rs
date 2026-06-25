use super::registry::{namespace_tool, parse_namespaced, GatewayRegistry};

#[test]
fn test_namespace_tool() {
    assert_eq!(namespace_tool("fs", "read_file"), "fs__read_file");
    assert_eq!(namespace_tool("brave", "search"), "brave__search");
}

#[test]
fn test_parse_namespaced_valid() {
    let result = parse_namespaced("fs__read_file");
    assert_eq!(result, Some(("fs".to_string(), "read_file".to_string())));
}

#[test]
fn test_parse_namespaced_no_separator() {
    assert_eq!(parse_namespaced("list_capabilities"), None);
}

#[test]
fn test_parse_namespaced_multiple_separators() {
    // "a__b__c" should split on first "__" -> ("a", "b__c")
    let result = parse_namespaced("server__tool__extra");
    assert_eq!(
        result,
        Some(("server".to_string(), "tool__extra".to_string()))
    );
}

#[test]
fn test_registry_new_is_empty() {
    let reg = GatewayRegistry::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let names = rt.block_on(reg.get_child_names());
    assert!(names.is_empty());
    let tools = rt.block_on(reg.get_merged_tools());
    assert!(tools.is_empty());
}
