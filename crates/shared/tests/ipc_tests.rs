use shared::models::*;

// ───────────────────────────── IpcRequest round-trips ─────────────────────────

#[test]
fn roundtrip_list() {
    let req = IpcRequest::List {
        offset: 5,
        limit: 20,
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::List { offset, limit } => {
            assert_eq!(offset, 5);
            assert_eq!(limit, 20);
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn roundtrip_search() {
    let req = IpcRequest::Search {
        query: "hello world".into(),
        limit: 10,
        offset: 0,
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::Search {
            query,
            limit,
            offset,
        } => {
            assert_eq!(query, "hello world");
            assert_eq!(limit, 10);
            assert_eq!(offset, 0);
        }
        _ => panic!("expected Search"),
    }
}

#[test]
fn roundtrip_get() {
    let req = IpcRequest::Get {
        id: "abc-123".into(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::Get { id } => assert_eq!(id, "abc-123"),
        _ => panic!("expected Get"),
    }
}

#[test]
fn roundtrip_delete() {
    let req = IpcRequest::Delete { id: "x".into() };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::Delete { id } => assert_eq!(id, "x"),
        _ => panic!("expected Delete"),
    }
}

#[test]
fn roundtrip_bulk_delete() {
    let req = IpcRequest::BulkDelete {
        ids: vec!["a".into(), "b".into()],
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::BulkDelete { ids } => assert_eq!(ids, vec!["a", "b"]),
        _ => panic!("expected BulkDelete"),
    }
}

#[test]
fn roundtrip_toggle_pin() {
    let req = IpcRequest::TogglePin {
        id: "pin-me".into(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::TogglePin { id } => assert_eq!(id, "pin-me"),
        _ => panic!("expected TogglePin"),
    }
}

#[test]
fn roundtrip_paste() {
    let req = IpcRequest::Paste { id: "p".into() };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::Paste { id } => assert_eq!(id, "p"),
        _ => panic!("expected Paste"),
    }
}

#[test]
fn roundtrip_clear() {
    let json = serde_json::to_string(&IpcRequest::Clear).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, IpcRequest::Clear));
}

#[test]
fn roundtrip_status() {
    let json = serde_json::to_string(&IpcRequest::Status).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, IpcRequest::Status));
}

#[test]
fn roundtrip_add_tag() {
    let req = IpcRequest::AddTag {
        id: "item1".into(),
        tag: "important".into(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::AddTag { id, tag } => {
            assert_eq!(id, "item1");
            assert_eq!(tag, "important");
        }
        _ => panic!("expected AddTag"),
    }
}

#[test]
fn roundtrip_remove_tag() {
    let req = IpcRequest::RemoveTag {
        id: "item1".into(),
        tag: "old".into(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcRequest::RemoveTag { id, tag } => {
            assert_eq!(id, "item1");
            assert_eq!(tag, "old");
        }
        _ => panic!("expected RemoveTag"),
    }
}

#[test]
fn roundtrip_get_config() {
    let json = serde_json::to_string(&IpcRequest::GetConfig).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, IpcRequest::GetConfig));
}

// ───────────────────────────── IpcResponse round-trips ────────────────────────

#[test]
fn roundtrip_response_ok() {
    let resp = IpcResponse::Ok {
        message: "done".into(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: IpcResponse = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcResponse::Ok { message } => assert_eq!(message, "done"),
        _ => panic!("expected Ok"),
    }
}

#[test]
fn roundtrip_response_error() {
    let resp = IpcResponse::Error {
        message: "bad".into(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: IpcResponse = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcResponse::Error { message } => assert_eq!(message, "bad"),
        _ => panic!("expected Error"),
    }
}

#[test]
fn roundtrip_response_status() {
    let resp = IpcResponse::Status {
        uptime_secs: 100,
        total_items: 42,
        db_size_bytes: 8192,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: IpcResponse = serde_json::from_str(&json).unwrap();
    match parsed {
        IpcResponse::Status {
            uptime_secs,
            total_items,
            db_size_bytes,
        } => {
            assert_eq!(uptime_secs, 100);
            assert_eq!(total_items, 42);
            assert_eq!(db_size_bytes, 8192);
        }
        _ => panic!("expected Status"),
    }
}

// ───────────────────────────── ContentType ────────────────────────────────────

#[test]
fn content_type_round_trip() {
    for ct in [
        ContentType::PlainText,
        ContentType::Html,
        ContentType::Image,
        ContentType::RichText,
        ContentType::Files,
        ContentType::Uri,
    ] {
        let s = ct.as_str();
        let parsed: ContentType = s.parse().unwrap();
        assert_eq!(parsed, ct);
    }
}

#[test]
fn content_type_unknown_default() {
    let ct: ContentType = "something_weird".parse().unwrap();
    assert_eq!(ct, ContentType::PlainText);
}
