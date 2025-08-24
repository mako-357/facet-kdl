use facet::Facet;
use facet_kdl::KdlSerializer;
use facet_serialize::Serialize;

// Facetトレイトを実装した構造体（Serializeが自動実装される）
#[derive(Debug, Clone, Facet)]
struct SimpleProcess {
    #[facet(argument)]
    id: String,
    #[facet(property)]
    command: String,
    #[facet(property)]
    enabled: bool,
}

#[test]
fn test_serialize_simple() {
    let process = SimpleProcess {
        id: "web-server".to_string(),
        command: "/usr/bin/node".to_string(),
        enabled: true,
    };

    // KDLシリアライザーを直接使用
    let mut serializer = KdlSerializer::new();

    // ルートノードを設定
    serializer.current_node = Some(kdl::KdlNode::new("process"));

    // シリアライズ
    process
        .serialize(&mut serializer)
        .expect("Failed to serialize");

    // ドキュメントに追加
    if let Some(node) = serializer.current_node.take() {
        serializer.document.nodes_mut().push(node);
    }

    let kdl_string = serializer.into_string();
    println!("Serialized KDL:\n{}", kdl_string);

    // 基本的な検証
    assert!(kdl_string.contains("process"));
    // プロパティとして出力されるはず
    assert!(kdl_string.contains("id=") || kdl_string.contains("\"web-server\""));
    assert!(kdl_string.contains("command=") || kdl_string.contains("/usr/bin/node"));
}

#[test]
fn test_serialize_with_to_string() {
    let process = SimpleProcess {
        id: "redis-server".to_string(),
        command: "/usr/local/bin/redis-server".to_string(),
        enabled: false,
    };

    // to_string関数を使用
    let kdl_string = facet_kdl::to_string(&process).expect("Failed to serialize");
    println!("Serialized with to_string:\n{}", kdl_string);

    // 検証
    assert!(kdl_string.contains("root"));
    // フィールドが含まれているか確認
    assert!(kdl_string.contains("redis-server") || kdl_string.contains("id="));
}

#[test]
fn test_serialize_nested() {
    #[derive(Debug, Clone, Facet)]
    struct Config {
        #[facet(child)]
        process: Process,
    }

    #[derive(Debug, Clone, Facet)]
    struct Process {
        #[facet(argument)]
        name: String,
        #[facet(property)]
        port: i32,
    }

    let config = Config {
        process: Process {
            name: "api-server".to_string(),
            port: 8080,
        },
    };

    let kdl_string = facet_kdl::to_string(&config).expect("Failed to serialize");
    println!("Nested structure KDL:\n{}", kdl_string);

    // ネストした構造が含まれているか確認
    assert!(kdl_string.contains("process") || kdl_string.contains("api-server"));
}
