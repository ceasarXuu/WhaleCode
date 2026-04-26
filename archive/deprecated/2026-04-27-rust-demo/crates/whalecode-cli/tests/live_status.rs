use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    thread::{self, JoinHandle},
};

use tempfile::tempdir;

#[test]
fn whale_live_run_prints_tool_permission_patch_and_usage_status() {
    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
    std::fs::write(repo.path().join("README.md"), "Fixture\n").expect("write readme");
    let session_path = repo.path().join("session.jsonl");
    let mock = MockDeepSeek::start(vec![edit_file_tool_call_sse(), final_answer_sse()]);

    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args([
            "run",
            "change the README fixture text",
            "--cwd",
            repo.path().to_str().expect("repo path"),
            "--session",
            session_path.to_str().expect("session path"),
            "--allow-write",
            "--max-turns",
            "3",
        ])
        .env("DEEPSEEK_API_KEY", "test-key")
        .env("DEEPSEEK_BASE_URL", mock.base_url())
        .env("WHALE_SECRET_HOME", whale_home.path())
        .output()
        .expect("run whale");

    mock.join();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("workspace:"));
    assert!(stdout.contains("model: deepseek-v4-flash"));
    assert!(stdout.contains("edit_file: enabled"));
    assert!(stdout.contains("run_command: disabled"));
    assert!(stdout.contains("session:"));
    assert_eq!(stdout.matches("session:").count(), 1);
    assert!(stdout.contains("tool: edit_file README.md"));
    assert!(stdout.contains("permission: edit_file allowed"));
    assert!(stdout.contains("modified: README.md"));
    assert!(stdout.contains("tool: edit_file README.md -> patch applied"));
    assert!(stdout.contains("Updated README."));
    assert!(stdout.contains("input tokens: 30"));
    assert!(stdout.contains("output tokens: 5"));
    assert!(stdout.contains("cached input tokens: 9"));
    assert!(stdout.contains("tool calls: 1"));
    assert!(stdout.contains("files changed: 1 (README.md)"));
    assert!(stdout.contains("duration:"));
    assert_eq!(
        std::fs::read_to_string(repo.path().join("README.md")).expect("read readme"),
        "Changed\n"
    );
}

#[test]
fn whale_live_run_can_create_file_in_empty_workspace() {
    let repo = tempdir().expect("repo");
    let whale_home = tempdir().expect("whale home");
    let session_path = repo.path().join("session.jsonl");
    let mock = MockDeepSeek::start(vec![write_file_tool_call_sse(), final_write_answer_sse()]);

    let output = Command::new(env!("CARGO_BIN_EXE_whale"))
        .args([
            "run",
            "create a tiny html page",
            "--cwd",
            repo.path().to_str().expect("repo path"),
            "--session",
            session_path.to_str().expect("session path"),
            "--allow-write",
            "--max-turns",
            "3",
        ])
        .env("DEEPSEEK_API_KEY", "test-key")
        .env("DEEPSEEK_BASE_URL", mock.base_url())
        .env("WHALE_SECRET_HOME", whale_home.path())
        .output()
        .expect("run whale");

    mock.join();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("tool: write_file index.html"));
    assert!(stdout.contains("permission: write_file allowed"));
    assert!(stdout.contains("modified: index.html"));
    assert!(stdout.contains("Wrote index.html."));
    assert!(stdout.contains("files changed: 1 (index.html)"));
    assert_eq!(
        std::fs::read_to_string(repo.path().join("index.html")).expect("read index"),
        "<!doctype html>\n<title>Whale</title>\n"
    );
}

struct MockDeepSeek {
    base_url: String,
    handle: JoinHandle<()>,
}

impl MockDeepSeek {
    fn start(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
        let handle = thread::spawn(move || {
            for response_body in responses {
                let (stream, _) = listener.accept().expect("accept request");
                respond(stream, &response_body);
            }
        });
        Self { base_url, handle }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn join(self) {
        self.handle.join().expect("mock server thread");
    }
}

fn respond(mut stream: TcpStream, body: &str) {
    read_http_request(&mut stream);
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response");
    stream.flush().expect("flush response");
}

fn read_http_request(stream: &mut TcpStream) {
    let mut request = Vec::new();
    let mut buf = [0_u8; 1024];
    let header_end = loop {
        let read = stream.read(&mut buf).expect("read request");
        assert_ne!(read, 0, "client closed request before headers completed");
        request.extend_from_slice(&buf[..read]);
        if let Some(index) = find_header_end(&request) {
            break index;
        }
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = content_length(&headers);
    let body_start = header_end + 4;
    let mut body_read = request.len().saturating_sub(body_start);
    while body_read < content_length {
        let read = stream.read(&mut buf).expect("read request body");
        assert_ne!(read, 0, "client closed request before body completed");
        body_read += read;
    }
}

fn find_header_end(request: &[u8]) -> Option<usize> {
    request.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content-length"))
        })
        .unwrap_or(0)
}

fn edit_file_tool_call_sse() -> String {
    r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"edit_file","arguments":"{\"path\":\"README.md\",\"old_string\":\"Fixture\",\"new_string\":\"Changed\"}"}}]},"finish_reason":null,"index":0}]}

data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":2,"total_tokens":12,"prompt_cache_hit_tokens":4}}

data: [DONE]

"#
    .to_owned()
}

fn final_answer_sse() -> String {
    r#"data: {"choices":[{"delta":{"content":"Updated README."},"finish_reason":null,"index":0}]}

data: {"choices":[],"usage":{"prompt_tokens":20,"completion_tokens":3,"total_tokens":23,"prompt_cache_hit_tokens":5}}

data: [DONE]

"#
    .to_owned()
}

fn write_file_tool_call_sse() -> String {
    r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"write_file","arguments":"{\"path\":\"index.html\",\"content\":\"<!doctype html>\\n<title>Whale</title>\\n\"}"}}]},"finish_reason":null,"index":0}]}

data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":2,"total_tokens":12,"prompt_cache_hit_tokens":4}}

data: [DONE]

"#
    .to_owned()
}

fn final_write_answer_sse() -> String {
    r#"data: {"choices":[{"delta":{"content":"Wrote index.html."},"finish_reason":null,"index":0}]}

data: {"choices":[],"usage":{"prompt_tokens":20,"completion_tokens":3,"total_tokens":23,"prompt_cache_hit_tokens":5}}

data: [DONE]

"#
    .to_owned()
}
