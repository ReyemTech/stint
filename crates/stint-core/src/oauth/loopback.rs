//! Ephemeral 127.0.0.1 HTTP listener for OAuth redirect capture.

use crate::{Error, Result};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::timeout;

fn success_html(provider_label: &str) -> String {
    format!(
        "<!doctype html><meta charset=utf-8>\
<title>stint — signed in</title>\
<style>body{{font:16px system-ui;padding:48px;max-width:520px;color:#1a1a1a}}</style>\
<h1>Signed in to {label}</h1>\
<p>You can close this tab and return to stint.</p>",
        label = html_escape(provider_label),
    )
}

fn error_html(provider_label: &str) -> String {
    format!(
        "<!doctype html><meta charset=utf-8>\
<title>stint — sign-in failed</title>\
<style>body{{font:16px system-ui;padding:48px;max-width:520px;color:#1a1a1a}}</style>\
<h1>Sign-in to {label} failed</h1>\
<p>Return to stint for details.</p>",
        label = html_escape(provider_label),
    )
}

/// Minimal HTML-attribute-safe escape — provider_label values come from
/// our own code ("Solidtime", "Google"), but escape defensively so a
/// future caller can pass any string without HTML injection.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Debug)]
pub struct CapturedCallback {
    pub code: String,
    pub state: String,
}

pub struct LoopbackServer {
    port: u16,
    rx: oneshot::Receiver<Result<CapturedCallback>>,
    timeout: Duration,
}

impl LoopbackServer {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn await_callback(self) -> Result<CapturedCallback> {
        match timeout(self.timeout, self.rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::OAuthCancelled),
            Err(_) => Err(Error::OAuthCancelled),
        }
    }
}

pub async fn listen_for_callback(
    server_timeout: Duration,
    provider_label: &str,
) -> Result<LoopbackServer> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| Error::OAuthLoopback(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| Error::OAuthLoopback(e.to_string()))?
        .port();
    let (tx, rx) = oneshot::channel();
    let provider_label = provider_label.to_string();

    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            let _ = tx.send(Err(Error::OAuthCancelled));
            return;
        };

        // Read the request line: "GET /callback?... HTTP/1.1\r\n..."
        let mut reader = BufReader::new(&mut socket);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).await.is_err() {
            let _ = tx.send(Err(Error::OAuthCancelled));
            return;
        }

        let parse_result = parse_callback_query(&request_line);
        let (body, response) = match &parse_result {
            Ok(_) => (success_html(&provider_label), "HTTP/1.1 200 OK"),
            Err(_) => (error_html(&provider_label), "HTTP/1.1 400 Bad Request"),
        };

        let payload = format!(
            "{response}\r\nContent-Type: text/html; charset=utf-8\r\n\
Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(payload.as_bytes()).await;
        let _ = socket.shutdown().await;

        let _ = tx.send(parse_result);
    });

    Ok(LoopbackServer {
        port,
        rx,
        timeout: server_timeout,
    })
}

/// Parse "GET /callback?code=...&state=... HTTP/1.1\r\n" → CapturedCallback.
fn parse_callback_query(request_line: &str) -> Result<CapturedCallback> {
    let path_and_query = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| Error::OAuthServer("malformed HTTP request line".into()))?;
    let query = path_and_query
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or_default();

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_desc = None;
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        let v = percent_decode(v);
        match k {
            "code" => code = Some(v),
            "state" => state = Some(v),
            "error" => error = Some(v),
            "error_description" => error_desc = Some(v),
            _ => {}
        }
    }

    if let Some(e) = error {
        let msg = match error_desc {
            Some(d) => format!("{e}: {d}"),
            None => e,
        };
        return Err(Error::OAuthServer(msg));
    }

    let code = code.ok_or_else(|| Error::OAuthServer("callback missing code".into()))?;
    let state = state.ok_or_else(|| Error::OAuthServer("callback missing state".into()))?;
    Ok(CapturedCallback { code, state })
}

fn percent_decode(s: &str) -> String {
    // Minimal `+` and `%XX` decode — sufficient for OAuth params.
    let mut out = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'+' => out.push(' '),
            b'%' => {
                let h1 = bytes.next();
                let h2 = bytes.next();
                if let (Some(h1), Some(h2)) = (h1, h2) {
                    if let (Some(d1), Some(d2)) =
                        ((h1 as char).to_digit(16), (h2 as char).to_digit(16))
                    {
                        out.push(((d1 * 16 + d2) as u8) as char);
                    }
                }
            }
            _ => out.push(b as char),
        }
    }
    out
}
