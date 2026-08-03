// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for the synchronous JSON-RPC 2.0 client.

use super::*;
use std::net::TcpListener;
use std::os::unix::net::UnixListener;

#[test]
fn connect_fails_for_nonexistent_socket() {
    let result = PrimalClient::connect(Path::new("/nonexistent/socket.sock"), "test");
    assert!(result.is_err());
    assert!(result.unwrap_err().is_connection_error());
}

#[test]
fn connect_tcp_fails_for_unreachable_addr() {
    let result = PrimalClient::connect_tcp("127.0.0.1:1", "test");
    assert!(result.is_err());
    assert!(result.unwrap_err().is_connection_error());
}

#[test]
fn connect_transport_tcp_implicit() {
    let result = PrimalClient::connect_transport("127.0.0.1:1", "test");
    assert!(result.is_err());
}

#[test]
fn connect_transport_tcp_explicit() {
    let result = PrimalClient::connect_transport("tcp:127.0.0.1:1", "test");
    assert!(result.is_err());
}

#[test]
fn connect_transport_unix_implicit() {
    let result = PrimalClient::connect_transport("/nonexistent/socket.sock", "test");
    assert!(result.is_err());
}

#[test]
fn connect_transport_unix_explicit() {
    let result = PrimalClient::connect_transport("unix:/nonexistent/socket.sock", "test");
    assert!(result.is_err());
}

#[test]
fn connect_transport_tcp_creates_tcp_transport() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];
        let response = format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"ok\":true}},\"id\":{id}}}\n");
        (&stream).write_all(response.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect_transport(&addr, "test").unwrap();
    assert!(client.transport.is_tcp());
    let resp = client
        .call("health.check", serde_json::Value::Null)
        .unwrap();
    assert!(resp.is_success());

    server.join().unwrap();
}

#[test]
fn round_trip_with_mock_server() {
    let dir = std::env::temp_dir().join("esotericwebb-test");
    std::fs::create_dir_all(&dir).unwrap();
    let sock_path = dir.join("test-roundtrip.sock");
    let _ = std::fs::remove_file(&sock_path);

    let listener = UnixListener::bind(&sock_path).unwrap();

    let sock_clone = sock_path.clone();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];

        let response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"status\":\"ok\"}},\"id\":{id}}}\n");
        (&stream).write_all(response.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect(&sock_clone, "test").unwrap();
    let resp = client
        .call("health.check", serde_json::Value::Null)
        .unwrap();
    assert!(resp.error.is_none());
    assert!(resp.result.is_some());

    server.join().unwrap();
    let _ = std::fs::remove_file(&sock_path);
}

#[test]
fn tcp_round_trip_with_mock_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];

        let response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"alive\":true}},\"id\":{id}}}\n");
        (&stream).write_all(response.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect_tcp(&addr, "test").unwrap();
    let resp = client
        .call("health.check", serde_json::Value::Null)
        .unwrap();
    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap()["alive"], true);

    server.join().unwrap();
}

#[test]
fn health_liveness_fallback_chain() {
    let dir = std::env::temp_dir().join("esotericwebb-test");
    std::fs::create_dir_all(&dir).unwrap();
    let sock_path = dir.join("test-health-fallback.sock");
    let _ = std::fs::remove_file(&sock_path);

    let listener = UnixListener::bind(&sock_path).unwrap();

    let sock_clone = sock_path.clone();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);

        for i in 0..4 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let req: serde_json::Value = serde_json::from_str(&line).unwrap();
            let id = &req["id"];
            let method = req["method"].as_str().unwrap_or("");

            let response = if i < 3 && method != "test.health" {
                format!(
                    "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32601,\"message\":\"not found\"}},\"id\":{id}}}\n"
                )
            } else {
                format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"alive\":true}},\"id\":{id}}}\n")
            };
            (&stream).write_all(response.as_bytes()).unwrap();
        }
    });

    let mut client = PrimalClient::connect(&sock_clone, "test").unwrap();
    let result = client.health_liveness().unwrap();
    assert!(result);

    server.join().unwrap();
    let _ = std::fs::remove_file(&sock_path);
}

#[test]
fn tcp_health_liveness_works() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();

        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];
        let response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"alive\":true}},\"id\":{id}}}\n");
        (&stream).write_all(response.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect_tcp(&addr, "test").unwrap();
    let result = client.health_liveness().unwrap();
    assert!(result);

    server.join().unwrap();
}

#[test]
fn primal_accessor_returns_name() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let _server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];
        let response = format!("{{\"jsonrpc\":\"2.0\",\"result\":null,\"id\":{id}}}\n");
        (&stream).write_all(response.as_bytes()).unwrap();
    });

    let client = PrimalClient::connect_tcp(&addr.to_string(), "myprimal").unwrap();
    assert_eq!(client.primal(), "myprimal");
}

#[test]
fn capabilities_returns_list() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];
        let response = format!(
            "{{\"jsonrpc\":\"2.0\",\"result\":{{\"methods\":[\"dag.query\",\"dag.append\"]}},\"id\":{id}}}\n"
        );
        (&stream).write_all(response.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect_tcp(&addr, "test").unwrap();
    let caps = client.capabilities().unwrap();
    assert!(caps["methods"].is_array());

    server.join().unwrap();
}

#[test]
fn capabilities_fallback_to_second_method() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);

        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];
        let response = format!(
            "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32601,\"message\":\"not found\"}},\"id\":{id}}}\n"
        );
        (&stream).write_all(response.as_bytes()).unwrap();

        let mut line2 = String::new();
        reader.read_line(&mut line2).unwrap();
        let req2: serde_json::Value = serde_json::from_str(&line2).unwrap();
        let id2 = &req2["id"];
        let response2 =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"caps\":[\"a\"]}},\"id\":{id2}}}\n");
        (&stream).write_all(response2.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect_tcp(&addr, "test").unwrap();
    let caps = client.capabilities().unwrap();
    assert!(caps["caps"].is_array());

    server.join().unwrap();
}

#[test]
fn default_timeout_returns_positive_duration() {
    let t = default_timeout();
    assert!(t.as_secs() > 0);
}

#[test]
fn transport_debug_uds() {
    let dir = std::env::temp_dir().join("esotericwebb-debug-test");
    std::fs::create_dir_all(&dir).unwrap();
    let sock_path = dir.join("debug.sock");
    let _ = std::fs::remove_file(&sock_path);

    let listener = UnixListener::bind(&sock_path).unwrap();
    let stream = UnixStream::connect(&sock_path).unwrap();
    let t = Transport::Uds(BufReader::new(stream));
    let debug = format!("{t:?}");
    assert_eq!(debug, "Transport::Uds");

    drop(listener);
    let _ = std::fs::remove_file(&sock_path);
}

#[test]
fn transport_debug_tcp() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stream = TcpStream::connect(addr).unwrap();
    let t = Transport::Tcp(BufReader::new(stream));
    let debug = format!("{t:?}");
    assert_eq!(debug, "Transport::Tcp");
}

#[test]
fn health_liveness_all_methods_fail_returns_false() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        for _ in 0..4 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let req: serde_json::Value = serde_json::from_str(&line).unwrap();
            let id = &req["id"];
            let response = format!(
                "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32601,\"message\":\"not found\"}},\"id\":{id}}}\n"
            );
            (&stream).write_all(response.as_bytes()).unwrap();
        }
    });

    let mut client = PrimalClient::connect_tcp(&addr, "test").unwrap();
    let result = client.health_liveness().unwrap();
    assert!(!result);

    server.join().unwrap();
}

#[test]
fn health_liveness_error_response_returns_false() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];
        let response = format!(
            "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32000,\"message\":\"internal error\"}},\"id\":{id}}}\n"
        );
        (&stream).write_all(response.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect_tcp(&addr, "test").unwrap();
    let result = client.health_liveness().unwrap();
    assert!(!result);

    server.join().unwrap();
}

#[test]
fn health_liveness_skips_invalid_params_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(&stream);

        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let req: serde_json::Value = serde_json::from_str(&line).unwrap();
        let id = &req["id"];
        let response = format!(
            "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32602,\"message\":\"invalid params\"}},\"id\":{id}}}\n"
        );
        (&stream).write_all(response.as_bytes()).unwrap();

        let mut line2 = String::new();
        reader.read_line(&mut line2).unwrap();
        let req2: serde_json::Value = serde_json::from_str(&line2).unwrap();
        let id2 = &req2["id"];
        let response2 =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"status\":\"healthy\"}},\"id\":{id2}}}\n");
        (&stream).write_all(response2.as_bytes()).unwrap();
    });

    let mut client = PrimalClient::connect_tcp(&addr, "test").unwrap();
    let result = client.health_liveness().unwrap();
    assert!(result, "should skip -32602 and succeed on next method");

    server.join().unwrap();
}
