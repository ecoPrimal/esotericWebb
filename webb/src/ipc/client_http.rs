// SPDX-License-Identifier: AGPL-3.0-or-later
//! HTTP/1.1 transport for primal communication.
//!
//! Two modes:
//! - **JSON-RPC POST**: For primals exposing `/jsonrpc` (songBird pattern).
//! - **REST GET health probe**: For primals exposing `/health` (petalTongue pattern).
//!
//! Zero external deps — raw `TcpStream` + manual HTTP/1.1 framing.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use super::envelope::{IpcError, JsonRpcRequest, JsonRpcResponse, classify_io_error};

/// Execute a single JSON-RPC request over HTTP/1.1 POST.
///
/// Opens a new TCP connection for each call (Connection: close). Parses
/// the HTTP response, extracts the JSON body, and deserializes into a
/// [`JsonRpcResponse`].
///
/// # Errors
///
/// Returns [`IpcError`] on connection failure, non-200 responses, or
/// body parse errors.
pub(super) fn call(
    addr: &str,
    path: &str,
    request: &JsonRpcRequest,
    timeout: std::time::Duration,
) -> Result<JsonRpcResponse, IpcError> {
    let body = serde_json::to_string(request).map_err(|e| IpcError::Serialization {
        detail: e.to_string(),
    })?;

    let http_request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        len = body.len(),
    );

    let mut stream = TcpStream::connect(addr).map_err(|e| classify_io_error(&e))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| classify_io_error(&e))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| classify_io_error(&e))?;
    stream
        .write_all(http_request.as_bytes())
        .map_err(|e| classify_io_error(&e))?;

    let mut reader = BufReader::new(&stream);
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|e| classify_io_error(&e))?;

    if !status_line.contains("200") {
        let code = status_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("?")
            .to_owned();
        return Err(IpcError::ProtocolError {
            detail: format!("HTTP {code} from {addr}{path}"),
        });
    }

    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|e| classify_io_error(&e))?;
        if header.trim().is_empty() {
            break;
        }
        if let Some(val) = header.strip_prefix("Content-Length:") {
            content_length = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = header.strip_prefix("content-length:") {
            content_length = val.trim().parse().unwrap_or(0);
        }
    }

    let response_body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        std::io::Read::read_exact(&mut reader, &mut buf).map_err(|e| classify_io_error(&e))?;
        String::from_utf8_lossy(&buf).into_owned()
    } else {
        let mut buf = String::new();
        reader
            .read_line(&mut buf)
            .map_err(|e| classify_io_error(&e))?;
        buf
    };

    if response_body.trim().is_empty() {
        return Err(IpcError::ProtocolError {
            detail: "empty HTTP response body".to_owned(),
        });
    }

    serde_json::from_str::<JsonRpcResponse>(response_body.trim()).map_err(|e| {
        IpcError::ProtocolError {
            detail: format!("HTTP response parse error: {e}"),
        }
    })
}

/// Probe a REST health endpoint via HTTP GET.
///
/// Returns `true` if the endpoint responds with HTTP 200 and a JSON body
/// containing `"status"` with a value indicating health (e.g. `"ok"`,
/// `"healthy"`, `"operational"`).
///
/// Handles both `Content-Length` and `Transfer-Encoding: chunked` responses.
/// This covers primals like petalTongue that expose `/health` as a REST
/// endpoint rather than JSON-RPC `health.check`.
pub(super) fn probe_rest_health(addr: &str, path: &str, timeout: std::time::Duration) -> bool {
    let http_request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Accept: application/json\r\n\
         Connection: close\r\n\
         \r\n"
    );

    let Ok(mut stream) = TcpStream::connect(addr) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    if stream.write_all(http_request.as_bytes()).is_err() {
        return false;
    }

    let mut reader = BufReader::new(&stream);
    let mut status_line = String::new();
    if reader.read_line(&mut status_line).is_err() {
        return false;
    }
    if !status_line.contains("200") {
        return false;
    }

    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
            return false;
        }
        if header.trim().is_empty() {
            break;
        }
        let lower = header.to_ascii_lowercase();
        if let Some(val) = lower.strip_prefix("content-length:") {
            content_length = val.trim().parse().ok();
        } else if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            chunked = true;
        }
    }

    let body = if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        if std::io::Read::read_exact(&mut reader, &mut buf).is_err() {
            return false;
        }
        String::from_utf8_lossy(&buf).into_owned()
    } else if chunked {
        read_chunked_body(&mut reader).unwrap_or_default()
    } else {
        let mut buf = String::new();
        let _ = reader.read_line(&mut buf);
        buf
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(body.trim()) else {
        return !body.trim().is_empty();
    };

    json.get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|s| matches!(s, "ok" | "healthy" | "operational" | "up" | "alive"))
}

/// Read a chunked transfer-encoding body.
fn read_chunked_body(reader: &mut BufReader<&TcpStream>) -> Option<String> {
    let mut body = String::new();
    loop {
        let mut size_line = String::new();
        reader.read_line(&mut size_line).ok()?;
        let chunk_size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
        if chunk_size == 0 {
            break;
        }
        let mut chunk_buf = vec![0u8; chunk_size];
        std::io::Read::read_exact(reader, &mut chunk_buf).ok()?;
        body.push_str(&String::from_utf8_lossy(&chunk_buf));
        let mut crlf = String::new();
        let _ = reader.read_line(&mut crlf);
    }
    Some(body)
}
