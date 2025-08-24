use facet::Facet;
use indoc::indoc;

#[test]
fn simple_string_value() {
    #[derive(Debug, Facet, PartialEq)]
    struct Config {
        #[facet(child)]
        name: Name,
    }

    #[derive(Debug, Facet, PartialEq)]
    struct Name {
        #[facet(argument)]
        value: String,
    }

    let kdl = indoc! {r#"
        name "test-value"
    "#};

    let result: Config = facet_kdl::from_str(kdl).unwrap();
    assert_eq!(result.name.value, "test-value");
}

#[test]
fn simple_integer_value() {
    #[derive(Debug, Facet, PartialEq)]
    struct Config {
        #[facet(argument)]
        count: i64,
    }

    let kdl = indoc! {r#"
        count 42
    "#};

    let result: Config = facet_kdl::from_str(kdl).unwrap();
    assert_eq!(result.count, 42);
}

#[test]
fn simple_bool_value() {
    #[derive(Debug, Facet, PartialEq)]
    struct Config {
        #[facet(argument)]
        enabled: bool,
    }

    let kdl = indoc! {r#"
        enabled true
    "#};

    let result: Config = facet_kdl::from_str(kdl).unwrap();
    assert_eq!(result.enabled, true);
}
