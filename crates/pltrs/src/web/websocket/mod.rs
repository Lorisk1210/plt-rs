//! The WebSocket protocol (RFC 6455), server side: the opening handshake,
//! [`frame`]s and [`message`]s.

pub(crate) mod frame;
pub(crate) mod message;

use pltrs_codec::{base64, sha1};

use super::http::{Request, Response, Status, Version};

pub(crate) use frame::{Opcode, WsError};
pub(crate) use message::{Message, MessageReader};

/// The GUID the handshake appends to the client's key (RFC 6455 section
/// 1.3).
const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The `Sec-WebSocket-Accept` value answering `Sec-WebSocket-Key` `key`.
pub(crate) fn accept_key(key: &str) -> String {
    base64::encode(&sha1::sha1(format!("{key}{GUID}").as_bytes()))
}

/// Whether `request` asks to become a WebSocket.
pub(crate) fn is_upgrade(request: &Request) -> bool {
    request.has_token("upgrade", "websocket")
}

/// The host and port of an `Origin` header (`scheme://host[:port]`).
fn origin_authority(origin: &str) -> Option<&str> {
    let (_, rest) = origin.split_once("://")?;
    Some(rest.split('/').next().unwrap_or(rest))
}

/// The answer to an upgrade request (RFC 6455 section 4.2): the 101
/// response, or the error response refusing it. Like Tornado, which
/// matplotlib's server runs on, a page from another origin may not open a
/// WebSocket.
pub(crate) fn handshake(request: &Request) -> Result<Response, Response> {
    let refuse = |status| Err(Response::error(status));
    if request.version < Version::Http11
        || request.header("host").is_none()
        || !request.has_token("connection", "upgrade")
    {
        return refuse(Status::BadRequest);
    }
    if request.header("sec-websocket-version") != Some("13") {
        return Err(
            Response::error(Status::UpgradeRequired).with_header("Sec-WebSocket-Version", "13")
        );
    }
    let Some(key) = request.header("sec-websocket-key") else {
        return refuse(Status::BadRequest);
    };
    if base64::decode(key).map(|nonce| nonce.len()) != Ok(16) {
        return refuse(Status::BadRequest);
    }
    if let Some(origin) = request.header("origin") {
        let same = origin_authority(origin)
            .zip(request.header("host"))
            .is_some_and(|(origin, host)| origin.eq_ignore_ascii_case(host));
        if !same {
            return refuse(Status::Forbidden);
        }
    }
    Ok(Response {
        status: Status::SwitchingProtocols,
        headers: vec![
            ("Upgrade", "websocket".to_owned()),
            ("Connection", "Upgrade".to_owned()),
            ("Sec-WebSocket-Accept", accept_key(key)),
        ],
        body: Vec::new().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::http::parse_request;
    use super::*;

    fn request(extra: &str) -> Request {
        let head = format!(
            "GET /1/ws HTTP/1.1\r\nHost: 127.0.0.1:8988\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n{extra}\r\n"
        );
        parse_request(head.as_bytes()).expect("a valid head")
    }

    #[test]
    fn rfc_6455_accept_key() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn a_valid_upgrade_is_accepted() {
        let response = handshake(&request(
            "Sec-WebSocket-Version: 13\r\nOrigin: http://127.0.0.1:8988\r\n",
        ))
        .expect("accepted");
        assert_eq!(response.status, Status::SwitchingProtocols);
        assert!(response.headers.contains(&(
            "Sec-WebSocket-Accept",
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".to_owned()
        )));
        assert!(is_upgrade(&request("")));
    }

    /// matplotlib's `test_websocket_rejects_cross_origin`: a page may open
    /// a WebSocket only from the origin it was served from, the host and
    /// port of `Origin` being those of `Host` (Tornado's `check_origin`).
    #[test]
    fn cross_origin_upgrades_are_forbidden() {
        let cases = [
            ("localhost:8988", "http://localhost:8988", true),
            ("localhost:8988", "http://evil.com", false),
            ("localhost:8988", "http://127.0.0.1:8988", false),
            ("localhost:8988", "http://[::1]:8988", false),
            ("127.0.0.1:8988", "http://127.0.0.1:8988", true),
            ("127.0.0.1:8988", "http://localhost:8988", false),
            ("127.0.0.1:8988", "http://[::1]:8988", false),
            ("[::1]:8988", "http://[::1]:8988", true),
            ("[::1]:8988", "http://[::2]:8988", false),
            ("[::1]:8988", "http://localhost:8988", false),
            ("[::1]:8988", "http://evil.com", false),
        ];
        for (host, origin, allowed) in cases {
            let head = format!(
                "GET /1/ws HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\n\
                 Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                 Sec-WebSocket-Version: 13\r\nOrigin: {origin}\r\n\r\n"
            );
            let request = parse_request(head.as_bytes()).expect("a valid head");
            let status = handshake(&request).map_or_else(|r| r.status, |r| r.status);
            let expected = if allowed {
                Status::SwitchingProtocols
            } else {
                Status::Forbidden
            };
            assert_eq!(status, expected, "Host {host}, Origin {origin}");
        }
    }

    #[test]
    fn bad_upgrades_are_refused() {
        let status = |extra: &str| handshake(&request(extra)).map_err(|r| r.status);
        assert_eq!(status(""), Err(Status::UpgradeRequired), "no version");
        assert_eq!(
            status("Sec-WebSocket-Version: 8\r\n"),
            Err(Status::UpgradeRequired)
        );
        assert_eq!(
            status("Sec-WebSocket-Version: 13\r\nOrigin: http://evil.example\r\n"),
            Err(Status::Forbidden)
        );
        assert_eq!(
            status("Sec-WebSocket-Version: 13\r\nOrigin: null\r\n"),
            Err(Status::Forbidden)
        );
        let short_key = parse_request(
            b"GET /1/ws HTTP/1.1\r\nHost: h\r\nUpgrade: websocket\r\nConnection: upgrade\r\n\
              Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: c2hvcnQ=\r\n\r\n",
        )
        .unwrap();
        assert_eq!(
            handshake(&short_key).map_err(|r| r.status),
            Err(Status::BadRequest)
        );
        let old = parse_request(
            b"GET /1/ws HTTP/1.0\r\nHost: h\r\nUpgrade: websocket\r\nConnection: upgrade\r\n\r\n",
        )
        .unwrap();
        assert_eq!(
            handshake(&old).map_err(|r| r.status),
            Err(Status::BadRequest)
        );
    }
}
