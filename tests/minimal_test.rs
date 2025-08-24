use facet::Facet;

#[test]
fn test_minimal_string() {
    // プロパティを持つシンプルな構造体
    #[derive(Debug, Facet, PartialEq)]
    struct SimpleString {
        #[facet(property)]
        value: String,
    }

    // KDLプロパティ形式
    let kdl = r#"value "hello""#;

    println!("Parsing minimal KDL: {}", kdl);

    match facet_kdl::from_str::<SimpleString>(kdl) {
        Ok(result) => {
            println!("Successfully parsed: {:?}", result);
            assert_eq!(result.value, "hello");
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
            panic!("Failed to parse minimal string");
        }
    }
}

#[test]
fn test_minimal_i32() {
    #[derive(Debug, Facet, PartialEq)]
    struct SimpleInt {
        count: i32,
    }

    let kdl = "count 42";

    println!("Parsing minimal int KDL: {}", kdl);

    match facet_kdl::from_str::<SimpleInt>(kdl) {
        Ok(result) => {
            println!("Successfully parsed: {:?}", result);
            assert_eq!(result.count, 42);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
            panic!("Failed to parse minimal int");
        }
    }
}
