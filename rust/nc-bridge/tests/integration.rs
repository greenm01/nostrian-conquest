use nc_bridge::server::create_router;
use nc_bridge::{BridgeConfig, NodeManager};
use nc_dropfile::{CallerInfo, CallerRole};
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_bridge_http_endpoints_and_events() {
    let temp_dir = std::env::temp_dir().join(format!("nc_bridge_integ_{}", std::process::id()));
    let static_dir = temp_dir.join("static");
    std::fs::create_dir_all(&static_dir).unwrap();
    std::fs::write(static_dir.join("index.html"), "<h1>Nostrian Conquest Gateway</h1>").unwrap();

    let config = BridgeConfig {
        bind_host: "127.0.0.1".to_string(),
        bind_port: 0,
        max_nodes: 4,
        node_base_dir: temp_dir.join("nodes"),
        static_dir: static_dir.clone(),
        ..BridgeConfig::default()
    };

    let manager = Arc::new(NodeManager::new(config));
    let app = create_router(manager.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    // 1. Health check endpoint
    let res = client.get(format!("{base_url}/health")).send().await.expect("health request");
    assert_eq!(res.status(), 200);
    let health: serde_json::Value = res.json().await.expect("health json");
    assert_eq!(health["status"], "ok");
    assert_eq!(health["service"], "nc-bridge");
    assert_eq!(health["active_sessions"], 0);

    // 2. Static file serving (GET /)
    let res = client.get(&base_url).send().await.expect("static index request");
    assert_eq!(res.status(), 200);
    let body = res.text().await.expect("body text");
    assert!(body.contains("Nostrian Conquest Gateway"));

    // 3. Prepare an active node session
    let node_idx = manager.allocate_node().await.expect("allocate node");
    let caller = CallerInfo {
        platform_id: "user_test_99".to_string(),
        handle: "admiral_vorn".to_string(),
        display_name: Some("Admiral Vorn".to_string()),
        public_key: None,
        role: CallerRole::Player,
        locale: Some("en-US".to_string()),
    };
    let session = manager.prepare_session(node_idx, 6001, caller, None).await.expect("prepare session");
    assert_eq!(manager.active_count().await, 1);

    // Verify health now shows 1 active session
    let res = client.get(format!("{base_url}/health")).send().await.expect("health request 2");
    let health: serde_json::Value = res.json().await.expect("health json 2");
    assert_eq!(health["active_sessions"], 1);

    // 4. Internal event: missing authorization token -> 401
    let event_payload = serde_json::json!({
        "summary": "Orbital bombardment completed at Sector 14"
    });
    let res = client
        .post(format!("{base_url}/internal/events/{node_idx}"))
        .json(&event_payload)
        .send()
        .await
        .expect("unauthorized event request");
    assert_eq!(res.status(), 401);

    // 5. Internal event: invalid token -> 403
    let res = client
        .post(format!("{base_url}/internal/events/{node_idx}"))
        .header("Authorization", "Bearer invalid_secret_token")
        .json(&event_payload)
        .send()
        .await
        .expect("forbidden event request");
    assert_eq!(res.status(), 403);

    // 6. Internal event: valid token -> 200 OK
    let res = client
        .post(format!("{base_url}/internal/events/{node_idx}"))
        .header("Authorization", format!("Bearer {}", session.callback_token))
        .json(&event_payload)
        .send()
        .await
        .expect("authorized event request");
    assert_eq!(res.status(), 200);
    let event_res: serde_json::Value = res.json().await.expect("event response json");
    assert_eq!(event_res["status"], "event_received");

    // Clean up
    manager.release_node(node_idx).await;
    assert_eq!(manager.active_count().await, 0);

    server_handle.abort();
    let _ = std::fs::remove_dir_all(&temp_dir);
}
