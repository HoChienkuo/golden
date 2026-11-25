use std::time::Duration;

use futures_util::future::BoxFuture;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, LOCATION, USER_AGENT};
use serde::Deserialize;
use serde_json::{Value, json};

use super::workspace::error;
use crate::{Error, Tool, ToolSpec};

const NAME: &str = "fetch";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 120_000;
const MAX_REDIRECTS: usize = 10;
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_MARKDOWN_CHARS: usize = 32 * 1024;

/// Fetches an `http` or `https` URL and returns its content as Markdown.
pub struct FetchTool {
    client: reqwest::Client,
}

impl Default for FetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FetchTool {
    /// Creates the tool with a client that does not follow redirects itself, so
    /// that every hop can be checked for an allowed scheme.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("default client configuration is valid"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    url: String,
    timeout_ms: Option<u64>,
}

impl Tool for FetchTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            NAME,
            "Fetch an http or https URL and return its content as Markdown. Redirects are followed up to 10 hops.",
            json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Absolute http or https URL to fetch." },
                    "timeout_ms": { "type": "integer", "minimum": 1, "description": "Request timeout in milliseconds. Defaults to 30000." }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        )
    }

    fn call(&self, args: Value) -> BoxFuture<'static, Result<String, Error>> {
        let client = self.client.clone();
        Box::pin(async move {
            if !args.is_object() {
                return Err(error(NAME, "arguments must be a JSON object"));
            }
            let args = serde_json::from_value(args).map_err(|err| error(NAME, err))?;
            fetch(&client, args).await
        })
    }
}

async fn fetch(client: &reqwest::Client, args: Arguments) -> Result<String, Error> {
    let timeout_ms = args.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(error(
            NAME,
            format!("timeout_ms must be at most {MAX_TIMEOUT_MS}"),
        ));
    }
    check_scheme(&args.url)
        .map_err(|err| error(NAME, format!("url is not a valid absolute URL: {err}")))?;

    let mut url = args.url.clone();
    let mut response;
    let mut status;
    let mut redirects = 0;
    loop {
        check_scheme(&url)?;
        let request = client
            .get(&url)
            .headers(request_headers())
            .timeout(Duration::from_millis(timeout_ms));
        response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                return Err(error(
                    NAME,
                    format!("GET {url} failed: {err}; check the URL and network access"),
                ));
            }
        };

        status = response.status();
        if status.is_redirection() {
            let Some(location) = response.headers().get(LOCATION) else {
                return Err(error(
                    NAME,
                    format!("{status} from {url} did not include a Location header"),
                ));
            };
            let location = location.to_str().map_err(|err| error(NAME, err))?;
            url = response
                .url()
                .join(location)
                .map_err(|err| error(NAME, err))?
                .to_string();
            redirects += 1;
            if redirects > MAX_REDIRECTS {
                return Err(error(
                    NAME,
                    format!(
                        "more than {MAX_REDIRECTS} redirects while fetching {}",
                        args.url
                    ),
                ));
            }
            continue;
        }
        if !status.is_success() {
            return Err(error(NAME, format!("GET {url} returned {status}")));
        }
        break;
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let final_url = response.url().as_str().to_owned();

    let mut bytes = Vec::new();
    while bytes.len() < MAX_RESPONSE_BYTES {
        let limit = (MAX_RESPONSE_BYTES - bytes.len()) as u64;
        let Some(chunk) = response.chunk().await.map_err(|err| error(NAME, err))? else {
            break;
        };
        if chunk.len() as u64 > limit {
            bytes.extend_from_slice(&chunk[..limit as usize]);
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    let truncated_body = !bytes.is_empty() && bytes.len() == MAX_RESPONSE_BYTES;

    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (content, kind) = if content_type.contains("html") {
        (markdown_from_html(&text), "markdown")
    } else if content_type.starts_with("text/")
        || content_type.contains("json")
        || content_type.is_empty()
    {
        (text, "text")
    } else {
        return Err(error(
            NAME,
            format!("{final_url} returned unsupported content type '{content_type}'"),
        ));
    };

    let truncated = truncated_body || content.chars().count() > MAX_MARKDOWN_CHARS;
    let content: String = content.chars().take(MAX_MARKDOWN_CHARS).collect();
    Ok(json!({
        "url": final_url,
        "status": status.as_u16(),
        "content_type": content_type,
        "kind": kind,
        "content": content,
        "truncated": truncated
    })
    .to_string())
}

fn check_scheme(url: &str) -> Result<(), Error> {
    let lowered = url.to_ascii_lowercase();
    if lowered.starts_with("http://") || lowered.starts_with("https://") {
        Ok(())
    } else {
        Err(error(NAME, "only http and https URLs are supported"))
    }
}

fn request_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,text/plain,*/*"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(concat!("golden-agent/", env!("CARGO_PKG_VERSION"))),
    );
    headers
}

/// Converts an HTML document into Markdown, keeping headings, lists, links,
/// code blocks and inline emphasis.
fn markdown_from_html(html: &str) -> String {
    let mut converter = MarkdownConverter::default();
    converter.feed(html);
    converter.finish()
}

#[derive(Default)]
struct MarkdownConverter {
    output: String,
    lines: Vec<String>,
    in_pre: bool,
    list_depth: usize,
    pending_prefix: Option<String>,
    block_break: bool,
}

const SKIPPED: [&str; 4] = ["script", "style", "head", "noscript"];
const BLOCKS: [&str; 10] = [
    "p",
    "div",
    "section",
    "article",
    "header",
    "footer",
    "aside",
    "main",
    "nav",
    "blockquote",
];

impl MarkdownConverter {
    fn feed(&mut self, html: &str) {
        let mut skipped_depth: usize = 0;
        for token in HtmlTokens::new(html) {
            match token {
                Token::Start(name, self_closing) => {
                    if SKIPPED.contains(&name.as_str()) && !self_closing {
                        skipped_depth += 1;
                        continue;
                    }
                    if skipped_depth > 0 {
                        continue;
                    }
                    self.start(&name, self_closing);
                }
                Token::End(name) => {
                    if SKIPPED.contains(&name.as_str()) {
                        skipped_depth = skipped_depth.saturating_sub(1);
                        continue;
                    }
                    if skipped_depth > 0 {
                        continue;
                    }
                    self.end(&name);
                }
                Token::Text(text) => {
                    if skipped_depth == 0 {
                        self.text(&text);
                    }
                }
            }
        }
    }

    fn finish(&mut self) -> String {
        // Flush any text still buffered for the current line. Blocks are
        // already separated by blank lines; code fences keep their own
        // single-newline layout.
        if !self.output.is_empty() {
            self.flush_line();
        }
        while self.lines.last().is_some_and(|line| line.is_empty()) {
            self.lines.pop();
        }
        self.lines.join("\n")
    }

    fn start(&mut self, name: &str, self_closing: bool) {
        if BLOCKS.contains(&name) {
            self.new_block();
        }
        if self.in_pre {
            return;
        }
        match name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = name[1..].parse::<usize>().expect("heading tag");
                self.new_block();
                self.output.push_str(&"#".repeat(level));
                self.output.push(' ');
            }
            "pre" => {
                self.new_block();
                self.in_pre = true;
                self.output.push_str("```");
                self.flush_line();
            }
            "code" if !self.in_pre => self.output.push('`'),
            "strong" | "b" => self.output.push_str("**"),
            "em" | "i" => self.output.push('*'),
            "del" | "s" | "strike" => self.output.push_str("~~"),
            "br" => self.flush_line(),
            "hr" => {
                self.new_block();
                self.output.push_str("---");
            }
            "li" => {
                // A list item starts a new line but stays within the list, so
                // items are not separated by blank lines.
                if !self.output.is_empty() {
                    self.flush_line();
                }
                self.pending_prefix = Some(format!(
                    "{}- ",
                    "  ".repeat(self.list_depth.saturating_sub(1))
                ));
            }
            "ol" | "ul" => {
                self.new_block();
                self.list_depth += 1;
            }
            "a" if !self_closing => self.output.push('['),
            _ => {}
        }
    }

    fn end(&mut self, name: &str) {
        match name {
            "pre" if self.in_pre => {
                let trimmed = self.output.trim_end_matches([' ', '\t']);
                self.output = trimmed.to_owned();
                self.flush_line();
                self.lines.push("```".to_owned());
                self.in_pre = false;
                self.block_break = true;
            }
            "code" if !self.in_pre => self.output.push('`'),
            "strong" | "b" => self.output.push_str("**"),
            "em" | "i" => self.output.push('*'),
            "del" | "s" | "strike" => self.output.push_str("~~"),
            "a" => self.output.push(']'),
            "ol" | "ul" => {
                self.new_block();
                self.list_depth = self.list_depth.saturating_sub(1);
            }
            _ if BLOCKS.contains(&name) => self.new_block(),
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        let decoded = decode_entities(text);
        if self.in_pre {
            for (index, line) in decoded.split('\n').enumerate() {
                if index > 0 {
                    self.flush_line();
                }
                self.output.push_str(line);
            }
            return;
        }
        let collapsed = collapse_whitespace(&decoded);
        if collapsed.is_empty() {
            return;
        }
        // Keep the separators around this text node so adjacent words are not
        // glued together across tags (e.g. the runs around a nested <strong>).
        if let Some(prefix) = self.pending_prefix.take() {
            self.output.push_str(&prefix);
        } else if decoded.starts_with(char::is_whitespace)
            && !self
                .output
                .ends_with(|ch: char| ch.is_whitespace() || ch == '>' || ch == '\n')
        {
            self.output.push(' ');
        }
        self.output.push_str(&collapsed);
        if decoded.ends_with(char::is_whitespace) {
            self.output.push(' ');
        }
    }

    fn flush_line(&mut self) {
        let line = self.output.trim_end();
        if self.block_break && !self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.lines.push(line.to_owned());
        self.output.clear();
        self.block_break = false;
    }

    fn new_block(&mut self) {
        if !self.output.is_empty() {
            self.flush_line();
        }
        self.block_break = true;
    }
}

/// Collapses runs of whitespace into single spaces, dropping a leading run that
/// would otherwise glue onto the previous block.
fn collapse_whitespace(text: &str) -> String {
    let mut result = String::new();
    let mut pending_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !result.is_empty() {
                result.push(' ');
            }
            pending_space = false;
            result.push(ch);
        }
    }
    result
}

fn decode_entities(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        result.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        let Some(end) = tail.find(';') else {
            result.push('&');
            rest = tail;
            continue;
        };
        let entity = &tail[..end];
        let decoded = if let Some(digits) = entity.strip_prefix("#x").or(entity.strip_prefix("#X"))
        {
            u32::from_str_radix(digits, 16)
                .ok()
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
        } else if let Some(digits) = entity.strip_prefix('#') {
            digits
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
        } else {
            match entity {
                "amp" => Some("&".to_owned()),
                "lt" => Some("<".to_owned()),
                "gt" => Some(">".to_owned()),
                "quot" => Some("\"".to_owned()),
                "apos" => Some("'".to_owned()),
                "nbsp" => Some(" ".to_owned()),
                "mdash" => Some("—".to_owned()),
                "ndash" => Some("–".to_owned()),
                "hellip" => Some("…".to_owned()),
                _ => None,
            }
        };
        match decoded {
            Some(decoded) => {
                result.push_str(&decoded);
                rest = &rest[start + end + 2..];
            }
            None => {
                result.push('&');
                rest = tail;
            }
        }
    }
    result.push_str(rest);
    result
}

enum Token {
    Start(String, bool),
    End(String),
    Text(String),
}

struct HtmlTokens<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> HtmlTokens<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// Splits the input at `<`, honouring quoted attribute values so that a `<`
    /// inside an attribute cannot start a tag.
    fn split_tag(&self, rest: &'a str) -> Option<(&'a str, &'a str)> {
        let mut quote = None;
        let bytes = rest.as_bytes();
        for (index, byte) in bytes.iter().enumerate() {
            match (quote, byte) {
                (Some(marker), value) if *value == marker => quote = None,
                (None, value @ (b'"' | b'\'')) => quote = Some(*value),
                (_, b'>') if quote.is_none() => {
                    return Some((&rest[..index], &rest[index + 1..]));
                }
                _ => {}
            }
        }
        None
    }

    fn parse_tag(raw: &str) -> Option<Token> {
        let raw = raw.trim_start_matches('<').trim_end();
        if raw.is_empty() || raw.starts_with('!') || raw.starts_with('?') {
            return None;
        }
        let closing = raw.strip_prefix('/').map(str::trim_start).unwrap_or(raw);
        // `strip_prefix` consumed the '/', so a leading '/' still means this is
        // a closing tag (e.g. `</pre/>`).
        let name_end = closing
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == ':'))
            .unwrap_or(closing.len());
        let name = closing[..name_end].to_ascii_lowercase();
        if name.is_empty() {
            return None;
        }
        Some(if raw.starts_with('/') {
            Token::End(name)
        } else {
            Token::Start(name, closing[name_end..].trim_end().ends_with('/'))
        })
    }
}

impl Iterator for HtmlTokens<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        loop {
            let rest = &self.input[self.position..];
            let Some(start) = rest.find('<') else {
                if rest.is_empty() {
                    return None;
                }
                self.position = self.input.len();
                return Some(Token::Text(rest.to_owned()));
            };
            if start > 0 {
                self.position += start;
                return Some(Token::Text(rest[..start].to_owned()));
            }
            let Some((tag, tail)) = self.split_tag(rest) else {
                self.position = self.input.len();
                return Some(Token::Text(rest.to_owned()));
            };
            self.position = self.input.len() - tail.len();
            if let Some(token) = Self::parse_tag(tag) {
                return Some(token);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::Tool;

    async fn spawn_server(responses: Vec<String>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            for response in responses {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                // Drain the request line and headers before replying.
                let mut buffer = [0u8; 4096];
                loop {
                    let read = socket.read(&mut buffer).await.unwrap_or(0);
                    if read == 0
                        || buffer[..read]
                            .windows(4)
                            .any(|window| window == b"\r\n\r\n")
                    {
                        break;
                    }
                }
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        address
    }

    fn http_response(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    async fn call(args: Value) -> Result<Value, String> {
        // The tests talk to a local server, so they must not go through an
        // ambient proxy (which would reject the loopback address).
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .expect("default client configuration is valid");
        let tool = FetchTool { client };
        match tool.call(args).await {
            Ok(text) => Ok(serde_json::from_str(&text).expect("tool result is JSON")),
            Err(Error::Tool { message, .. }) => Err(message),
            Err(err) => panic!("unexpected error variant: {err:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_converts_html_to_markdown() {
        let address = spawn_server(vec![http_response(concat!(
            "<html><head><style>p{color:red}</style><title>t</title></head>",
            "<body><h1>Title</h1><p>Hello   <strong>world</strong> &amp; friends.</p>",
            "<ul><li>one</li><li>two</li></ul>",
            "<pre>fn main() { println!(\"hi\"); }</pre>",
            "<a href=\"/next\">link</a></body></html>",
        ))])
        .await;

        let result = call(json!({ "url": format!("http://{address}/page") }))
            .await
            .unwrap();
        assert_eq!(result["status"], 200);
        assert_eq!(result["kind"], "markdown");
        assert_eq!(result["truncated"], false);
        let content = result["content"].as_str().unwrap();
        assert_eq!(
            content,
            "# Title\n\nHello **world** & friends.\n\n- one\n- two\n\n```\nfn main() { println!(\"hi\"); }\n```\n\n[link]"
        );
        assert!(!content.contains("color:red"));
    }

    #[tokio::test]
    async fn fetch_follows_redirects_and_returns_final_url() {
        let address = spawn_server(vec![
            "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_owned(),
            http_response("<p>arrived</p>"),
        ])
        .await;

        let result = call(json!({ "url": format!("http://{address}/start") }))
            .await
            .unwrap();
        assert_eq!(result["status"], 200);
        assert_eq!(result["url"], format!("http://{address}/final"));
        assert_eq!(result["content"], "arrived");
    }

    #[tokio::test]
    async fn fetch_rejects_unsupported_urls_and_errors() {
        assert!(
            call(json!({ "url": "ftp://example.com/file" }))
                .await
                .unwrap_err()
                .contains("only http and https")
        );

        let address = spawn_server(vec![
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
        ])
        .await;
        let message = call(json!({ "url": format!("http://{address}/missing") }))
            .await
            .unwrap_err();
        assert!(message.contains("404"));

        let message = call(json!({ "url": "http://127.0.0.1:9/", "timeout_ms": 500 }))
            .await
            .unwrap_err();
        assert!(message.contains("check the URL and network access"));
    }

    #[test]
    fn markdown_helpers_decode_and_collapse_text() {
        assert_eq!(collapse_whitespace(" a \n b\t"), "a b");
        assert_eq!(
            decode_entities("a &amp; b &#x4e2d; &bogus;"),
            "a & b 中 &bogus;"
        );
    }
}
