//! Lightweight browser viewer for the current TaskSpace.
//!
//! The TUI opens a localhost page instead of trying to render large task state in the terminal.
//! The page polls the `thread/taskspace/read` RPC, so it observes live runtime state
//! without adding a second persistence or event system.

use super::*;
use codex_app_server_protocol::ThreadTaskSpaceReadParams;
use codex_app_server_protocol::ThreadTaskSpaceReadResponse;
use serde_json::json;
use std::sync::atomic::AtomicI64;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

pub(super) const ACTION_MAP_VIEWER_NO_BROWSER_ENV: &str = "WHALE_ACTION_MAP_VIEWER_NO_BROWSER";

pub(super) struct ActionMapViewerServer {
    pub(super) thread_id: ThreadId,
    pub(super) url: String,
    task: JoinHandle<()>,
}

impl ActionMapViewerServer {
    fn new(thread_id: ThreadId, url: String, task: JoinHandle<()>) -> Self {
        Self {
            thread_id,
            url,
            task,
        }
    }

    pub(super) fn is_running_for(&self, thread_id: ThreadId) -> bool {
        self.thread_id == thread_id && !self.task.is_finished()
    }
}

impl Drop for ActionMapViewerServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct ViewerState {
    thread_id: ThreadId,
    request_handle: AppServerRequestHandle,
    next_request_id: AtomicI64,
}

impl App {
    pub(super) async fn open_action_map_viewer(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) -> Result<()> {
        let _ = app_server.thread_taskspace_read(thread_id).await?;
        let should_start = self
            .action_map_viewer
            .as_ref()
            .is_none_or(|viewer| !viewer.is_running_for(thread_id));
        if should_start {
            self.action_map_viewer =
                Some(start_action_map_viewer(thread_id, app_server.request_handle()).await?);
        }

        let Some(viewer) = self.action_map_viewer.as_ref() else {
            return Ok(());
        };
        self.open_action_map_viewer_url(viewer.url.clone());
        Ok(())
    }

    fn open_action_map_viewer_url(&mut self, url: String) {
        if cfg!(test) || std::env::var_os(ACTION_MAP_VIEWER_NO_BROWSER_ENV).is_some() {
            self.chat_widget
                .add_info_message(format!("TaskSpace viewer: {url}"), /*hint*/ None);
            return;
        }

        self.open_url_in_browser(url);
    }
}

async fn start_action_map_viewer(
    thread_id: ThreadId,
    request_handle: AppServerRequestHandle,
) -> Result<ActionMapViewerServer> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .wrap_err("failed to bind TaskSpace viewer to localhost")?;
    let addr = listener
        .local_addr()
        .wrap_err("failed to read TaskSpace viewer address")?;
    let url = format!("http://127.0.0.1:{}/", addr.port());
    let state = Arc::new(ViewerState {
        thread_id,
        request_handle,
        next_request_id: AtomicI64::new(1),
    });

    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                if let Err(err) = handle_viewer_connection(stream, state).await {
                    tracing::debug!(error = %err, "TaskSpace viewer request failed");
                }
            });
        }
    });

    Ok(ActionMapViewerServer::new(thread_id, url, task))
}

async fn handle_viewer_connection(mut stream: TcpStream, state: Arc<ViewerState>) -> Result<()> {
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream
        .read(&mut buffer)
        .await
        .wrap_err("failed to read TaskSpace viewer request")?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let path = request_path(&request);
    let response = match path.as_deref() {
        Some("/") => http_response("200 OK", "text/html; charset=utf-8", ACTION_MAP_VIEWER_HTML),
        Some("/snapshot.json") => {
            let body = fetch_snapshot_json(&state).await;
            http_response("200 OK", "application/json; charset=utf-8", &body)
        }
        _ => http_response("404 Not Found", "text/plain; charset=utf-8", "not found"),
    };

    stream
        .write_all(response.as_bytes())
        .await
        .wrap_err("failed to write TaskSpace viewer response")?;
    Ok(())
}

fn request_path(request: &str) -> Option<String> {
    let mut parts = request.lines().next()?.split_whitespace();
    let method = parts.next()?;
    let raw_path = parts.next()?;
    if method != "GET" {
        return None;
    }
    Some(raw_path.split('?').next().unwrap_or(raw_path).to_string())
}

async fn fetch_snapshot_json(state: &ViewerState) -> String {
    let request_id = RequestId::Integer(
        state
            .next_request_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    );
    let result: std::result::Result<ThreadTaskSpaceReadResponse, _> = state
        .request_handle
        .request_typed(ClientRequest::ThreadTaskSpaceRead {
            request_id,
            params: ThreadTaskSpaceReadParams {
                thread_id: state.thread_id.to_string(),
            },
        })
        .await;

    match result {
        Ok(response) => json!({
            "ok": true,
            "threadId": state.thread_id,
            "fetchedAtMs": now_ms(),
            "snapshot": response.snapshot,
        })
        .to_string(),
        Err(err) => json!({
            "ok": false,
            "threadId": state.thread_id,
            "fetchedAtMs": now_ms(),
            "error": err.to_string(),
        })
        .to_string(),
    }
}

fn http_response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

const ACTION_MAP_VIEWER_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>TaskSpace</title>
<style>
:root{color-scheme:light dark}
body{margin:24px;font:13px/1.45 Consolas,Menlo,monospace;background:Canvas;color:CanvasText}
h1{font-size:18px;margin:0 0 8px}
h2{font-size:15px;margin:24px 0 8px;border-bottom:1px solid #777;padding-bottom:4px}
h3{font-size:13px;margin:16px 0 6px}
.meta{color:#777;margin-bottom:16px}
.pill{display:inline-block;border:1px solid #777;padding:1px 6px;margin-right:6px}
table{width:100%;border-collapse:collapse;margin:8px 0 16px}
th,td{border:1px solid #777;padding:5px 7px;text-align:left;vertical-align:top}
th{font-weight:700}
pre{white-space:pre-wrap;margin:6px 0 0}
details{border:1px solid #777;padding:8px;margin:8px 0}
.error{color:#c00}
</style>
</head>
<body>
<h1>TaskSpace</h1>
<div id="meta" class="meta">loading...</div>
<div id="root"></div>
<script>
const root = document.getElementById('root');
const meta = document.getElementById('meta');
function el(tag, text, cls){const n=document.createElement(tag);if(cls)n.className=cls;if(text!==undefined)n.textContent=text;return n}
function row(values){const tr=el('tr');values.forEach(v=>tr.appendChild(el('td',v??'')));return tr}
function table(headers, rows){const t=el('table');const h=el('tr');headers.forEach(x=>h.appendChild(el('th',x)));t.appendChild(h);rows.forEach(r=>t.appendChild(row(r)));return t}
function list(value){return Array.isArray(value)&&value.length?value.join(', '):''}
function render(data){
  root.replaceChildren();
  if(!data.ok){root.appendChild(el('div',data.error||'failed to load snapshot','error'));return}
  const s=data.snapshot;
  meta.textContent=`thread ${data.threadId} | mode ${s.mode} | active ${s.activeMapId||'none'} | refreshed ${new Date(data.fetchedAtMs).toLocaleTimeString()}`;
  if(!s.maps.length){root.appendChild(el('p','No task path has been created in this thread.'));return}
  s.maps.forEach(m=>{
    root.appendChild(el('h2',`${m.title} (${m.id})`));
    const line=el('div');
    ['status '+m.status,'ready '+m.readyNodeCount,'running '+m.runningNodeCount,'completed '+m.completedNodeCount,'owner '+(m.ownerSessionId||'none'),'base '+m.baseMapVersion].forEach(x=>line.appendChild(el('span',x,'pill')));
    root.appendChild(line);
    root.appendChild(el('h3','nodes'));
    root.appendChild(table(['id','status','title','context','source refs','lease','results'],m.nodes.map(n=>[n.id,n.status,n.title,n.contextSummary,list(n.sourceRefs),n.activeLease||'',list(n.resultIds)])));
    if(m.edges.length){root.appendChild(el('h3','edges'));root.appendChild(table(['from','to'],m.edges.map(e=>[e.from,e.to])))}
    if(m.leases.length){root.appendChild(el('h3','leases'));root.appendChild(table(['id','node','agent thread','agent path'],m.leases.map(l=>[l.id,l.nodeId,l.agentThreadId||'',l.agentPath||''])))}
    if(m.results.length){
      root.appendChild(el('h3','results'));
      m.results.forEach(r=>{const d=el('details');d.appendChild(el('summary',`${r.id} | node ${r.nodeId} | ${r.kind} | ${new Date(r.createdAtMs).toLocaleTimeString()}`));d.appendChild(el('pre',r.body));root.appendChild(d)});
    }
  });
}
async function refresh(){
  try{render(await (await fetch('/snapshot.json',{cache:'no-store'})).json())}
  catch(e){root.replaceChildren(el('div',String(e),'error'))}
}
refresh();
setInterval(refresh,2000);
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_path_accepts_basic_get_paths() {
        assert_eq!(
            request_path("GET /snapshot.json?x=1 HTTP/1.1\r\nHost: localhost\r\n\r\n"),
            Some("/snapshot.json".to_string())
        );
        assert_eq!(request_path("POST /snapshot.json HTTP/1.1\r\n\r\n"), None);
    }

    #[test]
    fn viewer_html_contains_polling_snapshot_endpoint() {
        assert!(ACTION_MAP_VIEWER_HTML.contains("fetch('/snapshot.json'"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("setInterval(refresh,2000)"));
    }
}
