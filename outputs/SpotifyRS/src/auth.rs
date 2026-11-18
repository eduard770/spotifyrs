use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use url::Url;

pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
pub const SCOPES: &str = "user-read-playback-state user-modify-playback-state";
pub const LIBRARY_SCOPES: &str =
    "playlist-read-private playlist-read-collaborative user-library-read";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("O Client ID deve conter 32 caracteres hexadecimais. Não uses o Client Secret.")]
    ClientId,
    #[error("Não foi possível usar a porta local 8888. Fecha outra instância e tenta novamente.")]
    Listener,
    #[error("Não foi possível abrir o navegador predefinido.")]
    Browser,
    #[error("O início de sessão foi cancelado.")]
    Cancelled,
    #[error("O início de sessão expirou. Tenta novamente.")]
    Timeout,
    #[error("A autorização foi recusada no Spotify.")]
    Denied,
    #[error("A autorização guardada expirou. Volta a autorizar em Ligações.")]
    Expired,
    #[error("O Spotify recusou a autenticação (HTTP {0}). Verifica o Client ID e o endereço de retorno.")]
    Http(u16),
    #[error("Não foi possível contactar o serviço de autenticação. Verifica a ligação.")]
    Network,
    #[error("A resposta de autenticação não é válida.")]
    Response,
}

// Deliberadamente sem Debug/Serialize: credenciais nunca entram em logs ou ficheiros.
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Instant,
    pub scopes: &'static str,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    token_type: String,
    scope: Option<String>,
}

pub fn valid_client_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn credential_target(scopes: &str) -> &'static str {
    match scopes {
        SCOPES => "SpotifyRS/WebAPI",
        LIBRARY_SCOPES => "SpotifyRS/Library",
        _ => "SpotifyRS/Catalog",
    }
}

pub(crate) fn saved_token(client_id: &str, scopes: &str) -> Option<String> {
    let value = crate::credentials::read(credential_target(scopes))?;
    let (id, token) = value.split_once(':')?;
    (id == client_id && !token.is_empty()).then(|| token.to_owned())
}

pub(crate) fn save_session(client_id: &str, session: &Session) -> std::io::Result<()> {
    crate::credentials::write(
        credential_target(session.scopes),
        &format!("{client_id}:{}", session.refresh_token),
    )
}

pub(crate) fn forget_session(scopes: &str) -> std::io::Result<()> {
    crate::credentials::delete(credential_target(scopes))
}

pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn authorization_url(client_id: &str, state: &str, verifier: &str, scopes: &str) -> Url {
    let mut url = Url::parse("https://accounts.spotify.com/authorize").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("response_type", "code"),
        ("redirect_uri", REDIRECT_URI),
        ("state", state),
        ("code_challenge_method", "S256"),
        ("code_challenge", &challenge(verifier)),
    ]);
    if !scopes.is_empty() {
        url.query_pairs_mut().append_pair("scope", scopes);
    }
    url
}

#[derive(Debug, PartialEq)]
enum Callback {
    Code(String),
    Denied,
    Invalid,
}

fn parse_callback(request: &str, expected_state: &str) -> Callback {
    let Some(line) = request.lines().next() else {
        return Callback::Invalid;
    };
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() != 3
        || parts[0] != "GET"
        || !parts[1].starts_with("/callback?")
        || parts[2] != "HTTP/1.1"
    {
        return Callback::Invalid;
    }
    let Ok(url) = Url::parse(&format!("http://127.0.0.1:8888{}", parts[1])) else {
        return Callback::Invalid;
    };
    if url.path() != "/callback" {
        return Callback::Invalid;
    }
    let pairs: Vec<_> = url.query_pairs().collect();
    let unique = |name: &str| -> Option<String> {
        let mut values = pairs.iter().filter(|(key, _)| key == name);
        let value = values.next()?.1.to_string();
        if values.next().is_some() {
            None
        } else {
            Some(value)
        }
    };
    if unique("state").as_deref() != Some(expected_state) {
        return Callback::Invalid;
    }
    if unique("error").is_some() {
        return Callback::Denied;
    }
    match unique("code") {
        Some(code) if !code.is_empty() && code.len() <= 2048 => Callback::Code(code),
        _ => Callback::Invalid,
    }
}

pub fn login(
    client: &Client,
    client_id: &str,
    cancelled: &AtomicBool,
    scopes: &'static str,
) -> Result<Session, AuthError> {
    if !valid_client_id(client_id) {
        return Err(AuthError::ClientId);
    }
    // Bind antes de abrir o navegador: evita entregar códigos a outro processo na porta.
    let listener = TcpListener::bind("127.0.0.1:8888").map_err(|_| AuthError::Listener)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| AuthError::Listener)?;
    let verifier = random_secret();
    let state = random_secret();
    webbrowser::open(authorization_url(client_id, &state, &verifier, scopes).as_str())
        .map_err(|_| AuthError::Browser)?;
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AuthError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(AuthError::Timeout);
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
                let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                let mut request = Vec::new();
                let connection_deadline = Instant::now() + Duration::from_secs(2);
                let mut buffer = [0u8; 1024];
                while request.len() < 8192 && !request.windows(4).any(|x| x == b"\r\n\r\n") {
                    if Instant::now() >= connection_deadline || cancelled.load(Ordering::Relaxed) {
                        break;
                    }
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => request.extend_from_slice(&buffer[..n]),
                    }
                }
                let result = if request.len() <= 8192 && request.ends_with(b"\r\n\r\n") {
                    parse_callback(&String::from_utf8_lossy(&request), &state)
                } else {
                    Callback::Invalid
                };
                let (status, message) = match &result {
                    Callback::Code(_) => (
                        "200 OK",
                        "Autorizacao recebida. Podes fechar este separador e voltar a aplicacao.",
                    ),
                    Callback::Denied => {
                        ("200 OK", "Autorizacao cancelada. Podes voltar a aplicacao.")
                    }
                    Callback::Invalid => (
                        "400 Bad Request",
                        "Pedido invalido. Volta a aplicacao para iniciar sessao.",
                    ),
                };
                let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n{message}", message.len());
                match result {
                    Callback::Code(code) => {
                        if cancelled.load(Ordering::Relaxed) {
                            return Err(AuthError::Cancelled);
                        }
                        return token_request(
                            client,
                            &[
                                ("grant_type", "authorization_code"),
                                ("code", &code),
                                ("redirect_uri", REDIRECT_URI),
                                ("client_id", client_id),
                                ("code_verifier", &verifier),
                            ],
                            None,
                            scopes,
                        );
                    }
                    Callback::Denied => return Err(AuthError::Denied),
                    Callback::Invalid => continue,
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(_) => return Err(AuthError::Listener),
        }
    }
}

pub fn refresh(client: &Client, client_id: &str, session: &Session) -> Result<Session, AuthError> {
    token_request(
        client,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", &session.refresh_token),
            ("client_id", client_id),
        ],
        Some(&session.refresh_token),
        session.scopes,
    )
}

fn token_request(
    client: &Client,
    form: &[(&str, &str)],
    old_refresh: Option<&str>,
    scopes: &'static str,
) -> Result<Session, AuthError> {
    let response = client
        .post(TOKEN_URL)
        .form(form)
        .send()
        .map_err(|_| AuthError::Network)?;
    let status = response.status();
    if !status.is_success() && !(status == StatusCode::BAD_REQUEST && old_refresh.is_some()) {
        return Err(AuthError::Http(status.as_u16()));
    }
    let mut body = Vec::new();
    response
        .take(65537)
        .read_to_end(&mut body)
        .map_err(|_| AuthError::Network)?;
    if body.len() > 65536 {
        return Err(AuthError::Response);
    }
    parse_token_response(status, &body, old_refresh, scopes)
}

fn parse_token_response(
    status: StatusCode,
    body: &[u8],
    old_refresh: Option<&str>,
    scopes: &'static str,
) -> Result<Session, AuthError> {
    if !status.is_success() {
        #[derive(Deserialize)]
        struct ErrorResponse {
            error: String,
        }
        if status == StatusCode::BAD_REQUEST
            && old_refresh.is_some()
            && serde_json::from_slice::<ErrorResponse>(body)
                .is_ok_and(|response| response.error == "invalid_grant")
        {
            return Err(AuthError::Expired);
        }
        return Err(AuthError::Http(status.as_u16()));
    }
    let token: TokenResponse = serde_json::from_slice(body).map_err(|_| AuthError::Response)?;
    make_session(token, old_refresh, scopes)
}

fn make_session(
    token: TokenResponse,
    old_refresh: Option<&str>,
    scopes: &'static str,
) -> Result<Session, AuthError> {
    if !token.token_type.eq_ignore_ascii_case("bearer")
        || token.access_token.is_empty()
        || token.expires_in < 1
    {
        return Err(AuthError::Response);
    }
    if let Some(ref scope) = token.scope {
        if !scopes
            .split_whitespace()
            .all(|s| scope.split_whitespace().any(|granted| s == granted))
        {
            return Err(AuthError::Response);
        }
    }
    let refresh_token = token
        .refresh_token
        .or_else(|| old_refresh.map(str::to_owned))
        .filter(|v| !v.is_empty())
        .ok_or(AuthError::Response)?;
    Ok(Session {
        scopes,
        access_token: token.access_token,
        refresh_token,
        expires_at: Instant::now()
            + Duration::from_secs(token.expires_in.saturating_sub(60).clamp(1, 86400)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pkce_matches_rfc7636_vector() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
    #[test]
    fn rejects_csrf_duplicate_state_and_wrong_paths() {
        assert_eq!(
            parse_callback("GET /callback?code=x&state=wrong HTTP/1.1\r\n", "ok"),
            Callback::Invalid
        );
        assert_eq!(
            parse_callback("GET /callback?code=x&state=ok&state=ok HTTP/1.1\r\n", "ok"),
            Callback::Invalid
        );
        assert_eq!(
            parse_callback("GET /other?code=x&state=ok HTTP/1.1\r\n", "ok"),
            Callback::Invalid
        );
        assert_eq!(
            parse_callback("POST /callback?code=x&state=ok HTTP/1.1\r\n", "ok"),
            Callback::Invalid
        );
    }
    #[test]
    fn accepts_code_and_explicit_denial() {
        assert_eq!(
            parse_callback("GET /callback?code=a%2Bb&state=ok HTTP/1.1\r\n", "ok"),
            Callback::Code("a+b".into())
        );
        assert_eq!(
            parse_callback(
                "GET /callback?error=access_denied&state=ok HTTP/1.1\r\n",
                "ok"
            ),
            Callback::Denied
        );
    }
    #[test]
    fn refresh_retains_old_token_when_not_rotated() {
        let token = TokenResponse {
            access_token: "new".into(),
            refresh_token: None,
            expires_in: 3600,
            token_type: "Bearer".into(),
            scope: None,
        };
        assert_eq!(
            make_session(token, Some("old"), SCOPES)
                .unwrap()
                .refresh_token,
            "old"
        );
    }
    #[test]
    fn authorization_uses_pkce_without_secret() {
        let url = authorization_url("id", "state", "verifier", SCOPES);
        assert!(url
            .query_pairs()
            .any(|(k, v)| k == "code_challenge_method" && v == "S256"));
        assert!(!url.query_pairs().any(|(k, _)| k == "client_secret"));
    }
    #[test]
    fn distinguishes_revoked_refresh_token_from_other_authentication_errors() {
        let invalid_grant = br#"{"error":"invalid_grant"}"#;
        assert!(matches!(
            parse_token_response(StatusCode::BAD_REQUEST, invalid_grant, Some("old"), ""),
            Err(AuthError::Expired)
        ));
        assert!(matches!(
            parse_token_response(StatusCode::BAD_REQUEST, invalid_grant, None, ""),
            Err(AuthError::Http(400))
        ));
        assert!(matches!(
            parse_token_response(
                StatusCode::BAD_REQUEST,
                br#"{"error":"invalid_client"}"#,
                Some("old"),
                ""
            ),
            Err(AuthError::Http(400))
        ));
    }
}
