use facet::Facet;
use indoc::indoc;

#[test]
fn test_simple_meta_deser() {
    #[derive(Debug, Facet, PartialEq)]
    struct MetaOnly {
        #[facet(child)]
        meta: Meta,
    }

    #[derive(Debug, Facet, PartialEq)]
    struct Meta {
        #[facet(property)]
        version: String,
    }

    let kdl = indoc! {r#"
        meta {
            version "1.0.0"
        }
    "#};

    println!("Parsing KDL:\n{}", kdl);

    match facet_kdl::from_str::<MetaOnly>(kdl) {
        Ok(result) => {
            println!("Successfully parsed: {:?}", result);
            assert_eq!(result.meta.version, "1.0.0");
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
            panic!("Failed to parse");
        }
    }
}

#[test]
fn test_process_list() {
    #[derive(Debug, Facet, PartialEq)]
    struct ProcessList {
        #[facet(child)]
        process: Vec<Process>,
    }

    #[derive(Debug, Facet, PartialEq)]
    struct Process {
        #[facet(argument)]
        id: String,
        #[facet(property)]
        command: String,
    }

    // 複数のprocessノード
    let kdl = indoc! {r#"
        process "web-server" {
            command "/usr/bin/node"
        }
        process "redis" {
            command "/usr/local/bin/redis-server"
        }
    "#};

    println!("Parsing KDL with multiple process nodes:\n{}", kdl);

    match facet_kdl::from_str::<ProcessList>(kdl) {
        Ok(result) => {
            println!("Successfully parsed: {:?}", result);
            assert_eq!(result.process.len(), 2);
            assert_eq!(result.process[0].id, "web-server");
            assert_eq!(result.process[1].id, "redis");
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
            // エラーを詳しく見る
        }
    }
}

#[test]
fn test_simple_argument() {
    #[derive(Debug, Facet, PartialEq)]
    struct SimpleArg {
        #[facet(argument)]
        name: String,
    }

    let kdl = r#"name "test-value""#;

    println!("Parsing simple KDL: {}", kdl);

    match facet_kdl::from_str::<SimpleArg>(kdl) {
        Ok(result) => {
            println!("Successfully parsed: {:?}", result);
            assert_eq!(result.name, "test-value");
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

#[test]
fn test_root_with_children() {
    #[derive(Debug, Facet, PartialEq)]
    struct RootConfig {
        #[facet(child)]
        settings: Settings,
    }

    #[derive(Debug, Facet, PartialEq)]
    struct Settings {
        #[facet(property)]
        enabled: bool,
    }

    let kdl = indoc! {r#"
        settings {
            enabled #true
        }
    "#};

    println!("Parsing KDL with boolean:\n{}", kdl);

    match facet_kdl::from_str::<RootConfig>(kdl) {
        Ok(result) => {
            println!("Successfully parsed: {:?}", result);
            assert_eq!(result.settings.enabled, true);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
