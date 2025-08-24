use facet::Facet;
use facet_serialize::Serialize;
use std::collections::HashMap;

#[test]
fn test_complex_structure() {
    #[derive(Debug, Clone, Facet)]
    struct ProcessConfig {
        #[facet(argument)]
        id: String,
        #[facet(property)]
        command: String,
        #[facet(property)]
        args: Vec<String>,
        #[facet(child)]
        env: HashMap<String, String>,
    }

    let mut env = HashMap::new();
    env.insert("NODE_ENV".to_string(), "production".to_string());
    env.insert("PORT".to_string(), "3000".to_string());

    let process = ProcessConfig {
        id: "web-server".to_string(),
        command: "/usr/bin/node".to_string(),
        args: vec![
            "server.js".to_string(),
            "--port".to_string(),
            "3000".to_string(),
        ],
        env,
    };

    let kdl_string = facet_kdl::to_string(&process).expect("Failed to serialize");
    println!("Complex structure KDL:\n{}", kdl_string);

    // 検証
    assert!(kdl_string.contains("web-server"));
    assert!(kdl_string.contains("/usr/bin/node"));
}

#[test]
fn test_optional_fields() {
    #[derive(Debug, Clone, Facet)]
    struct OptionalConfig {
        #[facet(property)]
        name: String,
        #[facet(property)]
        description: Option<String>,
        #[facet(property)]
        count: Option<i32>,
    }

    let config_with_some = OptionalConfig {
        name: "test".to_string(),
        description: Some("A test config".to_string()),
        count: Some(42),
    };

    let kdl_with_some = facet_kdl::to_string(&config_with_some).expect("Failed to serialize");
    println!("With Some values:\n{}", kdl_with_some);

    let config_with_none = OptionalConfig {
        name: "test2".to_string(),
        description: None,
        count: None,
    };

    let kdl_with_none = facet_kdl::to_string(&config_with_none).expect("Failed to serialize");
    println!("With None values:\n{}", kdl_with_none);

    // 検証
    assert!(kdl_with_some.contains("test"));
    assert!(kdl_with_some.contains("A test config"));
    assert!(kdl_with_some.contains("42"));

    assert!(kdl_with_none.contains("test2"));
}
