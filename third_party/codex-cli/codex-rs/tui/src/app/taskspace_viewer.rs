//! Local browser viewer for the canonical TaskSpace snapshot.

use std::sync::Arc;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadTaskSpaceReadParams;
use codex_app_server_protocol::ThreadTaskSpaceReadResponse;
use codex_protocol::ThreadId;
use color_eyre::eyre::Result;
use color_eyre::eyre::WrapErr;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use super::App;
use crate::app_server_session::AppServerSession;

const NO_BROWSER_ENV: &str = "WHALE_TASKSPACE_VIEWER_NO_BROWSER";

pub(super) struct TaskSpaceViewerServer {
    thread_id: ThreadId,
    url: String,
    task: JoinHandle<()>,
}

impl TaskSpaceViewerServer {
    fn is_running_for(&self, thread_id: ThreadId) -> bool {
        self.thread_id == thread_id && !self.task.is_finished()
    }
}

impl Drop for TaskSpaceViewerServer {
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
    pub(super) async fn open_taskspace_viewer(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) -> Result<()> {
        app_server.thread_taskspace_read(thread_id).await?;
        if self
            .taskspace_viewer
            .as_ref()
            .is_none_or(|viewer| !viewer.is_running_for(thread_id))
        {
            self.taskspace_viewer =
                Some(start_taskspace_viewer(thread_id, app_server.request_handle()).await?);
        }

        let Some(viewer) = self.taskspace_viewer.as_ref() else {
            return Ok(());
        };
        let url = viewer.url.clone();
        if cfg!(test) || std::env::var_os(NO_BROWSER_ENV).is_some() {
            self.chat_widget
                .add_info_message(format!("TaskSpace viewer: {url}"), /*hint*/ None);
        } else {
            self.open_url_in_browser(url);
        }
        Ok(())
    }
}

async fn start_taskspace_viewer(
    thread_id: ThreadId,
    request_handle: AppServerRequestHandle,
) -> Result<TaskSpaceViewerServer> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .wrap_err("failed to bind TaskSpace viewer to localhost")?;
    let address = listener
        .local_addr()
        .wrap_err("failed to read TaskSpace viewer address")?;
    let url = format!("http://127.0.0.1:{}/", address.port());
    let state = Arc::new(ViewerState {
        thread_id,
        request_handle,
        next_request_id: AtomicI64::new(1),
    });
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                if let Err(error) = handle_connection(stream, state).await {
                    tracing::debug!(%error, "TaskSpace viewer request failed");
                }
            });
        }
    });
    Ok(TaskSpaceViewerServer {
        thread_id,
        url,
        task,
    })
}

async fn handle_connection(mut stream: TcpStream, state: Arc<ViewerState>) -> Result<()> {
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream
        .read(&mut buffer)
        .await
        .wrap_err("failed to read TaskSpace viewer request")?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let response = match request_path(&request) {
        Some("/") => http_response("200 OK", "text/html; charset=utf-8", VIEWER_HTML),
        Some("/snapshot.json") => http_response(
            "200 OK",
            "application/json; charset=utf-8",
            &fetch_snapshot_json(&state).await,
        ),
        _ => http_response("404 Not Found", "text/plain; charset=utf-8", "not found"),
    };
    stream
        .write_all(response.as_bytes())
        .await
        .wrap_err("failed to write TaskSpace viewer response")?;
    Ok(())
}

fn request_path(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    (parts.next()? == "GET").then_some(parts.next()?.split('?').next()?)
}

async fn fetch_snapshot_json(state: &ViewerState) -> String {
    let request_id = RequestId::Integer(state.next_request_id.fetch_add(1, Ordering::Relaxed));
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
            "snapshot": response.snapshot,
        })
        .to_string(),
        Err(error) => json!({
            "ok": false,
            "threadId": state.thread_id,
            "error": error.to_string(),
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

const VIEWER_HTML: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>TaskSpace</title><style>
:root{color-scheme:light dark}body{margin:24px;font:13px/1.45 ui-monospace,SFMono-Regular,Consolas,monospace;background:Canvas;color:CanvasText}
h1{font-size:20px;margin:0 0 6px}.meta{color:#777;margin-bottom:18px}.error{color:#c33}
.graph{display:grid;grid-template-columns:minmax(180px,1fr) minmax(220px,2fr) minmax(180px,1fr);gap:18px;align-items:start}
.lane{border:1px solid #777;padding:10px;min-height:120px}.lane h2{font-size:13px;margin:0 0 10px}.node{border:1px solid #777;padding:8px;margin:8px 0}.node strong{display:block}.refs{color:#777;margin-top:4px}
table{width:100%;border-collapse:collapse;margin-top:18px}th,td{border:1px solid #777;padding:6px;text-align:left;vertical-align:top}th{font-weight:700}
.pill{display:inline-block;border:1px solid #777;padding:1px 6px;margin:0 6px 8px 0}@media(max-width:780px){.graph{grid-template-columns:1fr}}
</style></head><body><h1>TaskSpace</h1><div id="meta" class="meta">loading...</div><main id="root"></main><script>
const root=document.getElementById('root'),meta=document.getElementById('meta');
const el=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n};
function nodeCard(node,label){const n=el('div',undefined,'node');n.append(el('strong',`${label}: ${node.nodeId}`),el('div',node.goal),el('div',`source refs: ${(node.sourceRefs||[]).join(', ')||'none'}`,'refs'));return n}
function lane(title,nodes,label){const box=el('section',undefined,'lane');box.append(el('h2',title));nodes.forEach(n=>box.append(nodeCard(n,label)));return box}
function table(headers,rows){const t=el('table'),head=el('tr');headers.forEach(h=>head.append(el('th',h)));t.append(head);rows.forEach(values=>{const row=el('tr');values.forEach(v=>row.append(el('td',String(v??''))));t.append(row)});return t}
function render(data){root.replaceChildren();if(!data.ok){root.append(el('div',data.error||'read failed','error'));return}const s=data.snapshot,m=s.map;meta.textContent=`thread ${data.threadId} | ${s.schemaVersion} | mode ${s.mode}`;if(!m){root.append(el('p','No canonical map has been initialized.'));return}
const facts=el('div');[`map ${m.mapId}`,`revision ${m.revision}`,`completions ${Object.keys(m.completionRecords).length}`,`blocks ${Object.keys(m.blockRecords).length}`,`reservations ${Object.keys(m.actionReservations).length}`,`terminal ${m.terminalRecord?'yes':'no'}`].forEach(x=>facts.append(el('span',x,'pill')));root.append(facts);
const graph=el('div',undefined,'graph');graph.append(lane('Root',[m.root],'root'),lane('Work',m.workNodes,'work'),lane('Finish',[m.finish],'finish'));root.append(graph);
root.append(table(['from','to'],m.edges.map(e=>[e.from,e.to])));
root.append(table(['reservation','node','action','tool','call index'],Object.entries(m.actionReservations).map(([id,r])=>[id,r.nodeId,r.actionId,r.toolName,r.responseCallIndex])));
root.append(table(['result','node','action','reservation','error'],Object.entries(m.resultRefs).map(([id,r])=>[id,r.nodeId,r.actionId,r.reservationId,r.isError])));
}
async function refresh(){try{render(await(await fetch('/snapshot.json',{cache:'no-store'})).json())}catch(error){root.replaceChildren(el('div',String(error),'error'))}}
refresh();setInterval(refresh,2000);
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_path_only_accepts_get() {
        assert_eq!(
            request_path("GET /snapshot.json?rev=2 HTTP/1.1\r\n\r\n"),
            Some("/snapshot.json")
        );
        assert_eq!(request_path("POST /snapshot.json HTTP/1.1\r\n\r\n"), None);
    }

    #[test]
    fn viewer_uses_canonical_snapshot_fields() {
        for field in ["mapId", "workNodes", "completionRecords", "terminalRecord"] {
            assert!(VIEWER_HTML.contains(field));
        }
        assert!(!VIEWER_HTML.contains("bootstrapRequired"));
    }
}
