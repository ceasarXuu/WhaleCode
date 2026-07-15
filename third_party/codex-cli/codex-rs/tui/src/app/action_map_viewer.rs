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
body{margin:20px;font:12px/1.45 Consolas,Menlo,monospace;background:Canvas;color:CanvasText}
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
summary{cursor:pointer}
.graph{position:relative;overflow:hidden;border:1px solid #777;margin:8px 0 14px;background:rgba(127,127,127,.06);height:clamp(340px,62vh,820px);cursor:grab;touch-action:none;user-select:none}
.graph.dragging{cursor:grabbing}
.graph-world{position:absolute;left:0;top:0;transform-origin:0 0}
.graph svg{position:absolute;inset:0;pointer-events:none}
.graph-controls{position:absolute;right:8px;top:8px;z-index:3;display:flex;gap:4px}
.graph-controls button{font:12px/1 Consolas,Menlo,monospace;border:1px solid #777;background:Canvas;color:CanvasText;padding:4px 7px}
.graph-help{position:absolute;left:8px;bottom:8px;color:#777;background:Canvas;padding:2px 5px;border:1px solid #777}
.graph-node{position:absolute;box-sizing:border-box;width:220px;min-height:76px;border:1px solid #777;background:Canvas;padding:7px}
.graph-node.running{border-color:#0a84ff}.graph-node.ready{border-color:#6a8f00}.graph-node.completed,.graph-node.closed{border-color:#2d8a4d}.graph-node.blocked{border-color:#b00020}
.node-title{font-weight:700;margin-bottom:5px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.node-meta{color:#777;margin-top:5px}
.edge{fill:none;stroke:#777;stroke-width:1.2}
.error{color:#c00}
@media(max-width:900px){.map-layout{grid-template-columns:1fr}}
</style>
</head>
<body>
<h1>TaskSpace</h1>
<div id="meta" class="meta">loading...</div>
<div id="root"></div>
<script>
const root = document.getElementById('root');
const meta = document.getElementById('meta');
const ui = {open:new Set(), scrollY:0, graph:new Map(), lastPayload:'', pending:null};
function el(tag, text, cls){const n=document.createElement(tag);if(cls)n.className=cls;if(text!==undefined)n.textContent=text;return n}
function row(values){const tr=el('tr');values.forEach(v=>tr.appendChild(el('td',v??'')));return tr}
function table(headers, rows){const t=el('table');const h=el('tr');headers.forEach(x=>h.appendChild(el('th',x)));t.appendChild(h);rows.forEach(r=>t.appendChild(row(r)));return t}
function list(value){return Array.isArray(value)&&value.length?value.join(', '):''}
function saveUi(){document.querySelectorAll('details[data-key]').forEach(d=>d.open?ui.open.add(d.dataset.key):ui.open.delete(d.dataset.key));ui.scrollY=window.scrollY}
function restoreUi(){document.querySelectorAll('details[data-key]').forEach(d=>{d.open=ui.open.has(d.dataset.key)});requestAnimationFrame(()=>window.scrollTo(0,ui.scrollY))}
document.addEventListener('toggle',e=>{const k=e.target&&e.target.dataset&&e.target.dataset.key;if(k){e.target.open?ui.open.add(k):ui.open.delete(k)}},true);
function detail(key, summary, body){const d=el('details');d.dataset.key=key;d.open=ui.open.has(key);d.appendChild(el('summary',summary));d.appendChild(body);return d}
function short(text, max){text=text||'';return text.length>max?text.slice(0,max-1)+'...':text}
function activeSelection(){const s=window.getSelection&&window.getSelection();return !!(s&&!s.isCollapsed&&String(s).length)}
function graphState(key){if(!ui.graph.has(key))ui.graph.set(key,{x:20,y:20,scale:1});return ui.graph.get(key)}
function applyGraphTransform(world, state){world.style.transform=`translate(${state.x}px,${state.y}px) scale(${state.scale})`}
function clamp(value,min,max){return Math.max(min,Math.min(max,value))}
function flushPending(){if(activeSelection()||!ui.pending)return;const data=ui.pending;ui.pending=null;renderIfChanged(data,true)}
document.addEventListener('selectionchange',()=>{setTimeout(flushPending,120)});
function graphLayout(nodes, edges){
  const ids=new Set(nodes.map(n=>n.id));
  const incoming=new Map(nodes.map(n=>[n.id,0]));
  const outgoing=new Map(nodes.map(n=>[n.id,[]]));
  edges.forEach(e=>{if(ids.has(e.from)&&ids.has(e.to)){incoming.set(e.to,(incoming.get(e.to)||0)+1);outgoing.get(e.from).push(e.to)}});
  const queue=nodes.filter(n=>(incoming.get(n.id)||0)===0).map(n=>n.id);
  const level=new Map(queue.map(id=>[id,0]));
  const remaining=new Map(incoming);
  for(let i=0;i<queue.length;i++){const id=queue[i];(outgoing.get(id)||[]).forEach(to=>{level.set(to,Math.max(level.get(to)||0,(level.get(id)||0)+1));remaining.set(to,(remaining.get(to)||0)-1);if(remaining.get(to)===0)queue.push(to)})}
  nodes.forEach(n=>{if(!level.has(n.id))level.set(n.id,0)});
  const columns=new Map();
  nodes.forEach(n=>{const l=level.get(n.id)||0;if(!columns.has(l))columns.set(l,[]);columns.get(l).push(n.id)});
  const pos=new Map(), w=220, h=76, gapX=70, gapY=28, pad=18;
  columns.forEach((ids,l)=>ids.forEach((id,i)=>pos.set(id,{x:pad+l*(w+gapX),y:pad+i*(h+gapY),w,h})));
  const maxLevel=Math.max(0,...Array.from(columns.keys()));
  const maxRows=Math.max(1,...Array.from(columns.values()).map(v=>v.length));
  return {pos,width:pad*2+(maxLevel+1)*w+maxLevel*gapX,height:pad*2+maxRows*h+(maxRows-1)*gapY};
}
function renderGraph(m){
  const g=el('div');g.className='graph';
  if(!m.nodes.length){g.appendChild(el('div','No nodes yet.','node-meta'));return g}
  const layout=graphLayout(m.nodes,m.edges);
  const key='graph:'+m.id;
  const state=graphState(key);
  const controls=el('div');controls.className='graph-controls';
  const zoomIn=el('button','+'),zoomOut=el('button','-'),reset=el('button','reset');
  controls.appendChild(zoomIn);controls.appendChild(zoomOut);controls.appendChild(reset);g.appendChild(controls);
  const inner=el('div');inner.className='graph-world';inner.style.width=layout.width+'px';inner.style.height=layout.height+'px';applyGraphTransform(inner,state);
  const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');
  svg.setAttribute('width',layout.width);svg.setAttribute('height',layout.height);
  const defs=document.createElementNS(svg.namespaceURI,'defs');
  const marker=document.createElementNS(svg.namespaceURI,'marker');marker.setAttribute('id','arrow');marker.setAttribute('markerWidth','8');marker.setAttribute('markerHeight','8');marker.setAttribute('refX','7');marker.setAttribute('refY','3');marker.setAttribute('orient','auto');
  const head=document.createElementNS(svg.namespaceURI,'path');head.setAttribute('d','M0,0 L7,3 L0,6 Z');head.setAttribute('fill','#777');marker.appendChild(head);defs.appendChild(marker);svg.appendChild(defs);
  m.edges.forEach(e=>{const a=layout.pos.get(e.from),b=layout.pos.get(e.to);if(!a||!b)return;const p=document.createElementNS(svg.namespaceURI,'path');const x1=a.x+a.w,y1=a.y+a.h/2,x2=b.x,y2=b.y+b.h/2,mid=(x1+x2)/2;p.setAttribute('class','edge');p.setAttribute('marker-end','url(#arrow)');p.setAttribute('d',`M ${x1} ${y1} C ${mid} ${y1}, ${mid} ${y2}, ${x2} ${y2}`);svg.appendChild(p)});
  inner.appendChild(svg);
  m.nodes.forEach(n=>{const p=layout.pos.get(n.id);const card=el('div');card.className='graph-node '+(n.status||'');card.style.left=p.x+'px';card.style.top=p.y+'px';card.appendChild(el('div',short(n.goal||n.id,32),'node-title'));card.appendChild(el('div',n.id+' | '+(n.role||'')+' | '+(n.status||''),'node-meta'));card.appendChild(el('div','results '+((n.resultIds||[]).length)+' | '+(n.activeLease?'leased':'free'),'node-meta'));inner.appendChild(card)});
  g.appendChild(inner);g.appendChild(el('div','drag to pan | wheel to zoom','graph-help'));
  function zoomAt(factor, cx, cy){
    const rect=g.getBoundingClientRect(), px=cx-rect.left, py=cy-rect.top;
    const beforeX=(px-state.x)/state.scale,beforeY=(py-state.y)/state.scale;
    state.scale=clamp(state.scale*factor,.35,2.8);
    state.x=px-beforeX*state.scale;state.y=py-beforeY*state.scale;applyGraphTransform(inner,state);
  }
  zoomIn.addEventListener('click',e=>{e.stopPropagation();const r=g.getBoundingClientRect();zoomAt(1.18,r.left+r.width/2,r.top+r.height/2)});
  zoomOut.addEventListener('click',e=>{e.stopPropagation();const r=g.getBoundingClientRect();zoomAt(1/1.18,r.left+r.width/2,r.top+r.height/2)});
  reset.addEventListener('click',e=>{e.stopPropagation();state.x=20;state.y=20;state.scale=1;applyGraphTransform(inner,state)});
  g.addEventListener('wheel',e=>{e.preventDefault();zoomAt(e.deltaY<0?1.12:1/1.12,e.clientX,e.clientY)},{passive:false});
  let drag=null;
  g.addEventListener('pointerdown',e=>{if(e.target.closest('button'))return;drag={x:e.clientX,y:e.clientY,baseX:state.x,baseY:state.y};g.classList.add('dragging');g.setPointerCapture(e.pointerId)});
  g.addEventListener('pointermove',e=>{if(!drag)return;state.x=drag.baseX+e.clientX-drag.x;state.y=drag.baseY+e.clientY-drag.y;applyGraphTransform(inner,state)});
  function endDrag(e){if(!drag)return;drag=null;g.classList.remove('dragging');try{g.releasePointerCapture(e.pointerId)}catch{}}
  g.addEventListener('pointerup',endDrag);g.addEventListener('pointercancel',endDrag);
  return g;
}
function renderResultDetail(r){
  const box=el('div');
  box.appendChild(table(['field','value'],[
    ['assignment',r.assignmentId],
    ['kind',r.kind],
    ['action class',r.actionClass||''],
    ['tool success',r.toolSuccess===null?'':String(r.toolSuccess)],
    ['source event',r.sourceEventRef],
    ['source thread',r.sourceThreadId],
    ['artifacts',list(r.artifactRefs)],
  ]));
  return box;
}
function renderIfChanged(data, force){
  const payload=JSON.stringify(data);
  if(!force&&payload===ui.lastPayload)return;
  if(activeSelection()){ui.pending=data;return}
  ui.lastPayload=payload;render(data);
}
function render(data){
  saveUi();
  const next=el('div');
  if(!data.ok){next.appendChild(el('div',data.error||'failed to load snapshot','error'));root.replaceChildren(next);restoreUi();return}
  const s=data.snapshot;
  const m=s.map;
  meta.textContent=`thread ${data.threadId} | schema ${s.schemaVersion} | mode ${s.mode} | bootstrap ${s.bootstrapRequired?'required':'ok'} | map ${m?m.id:'none'} | refreshed ${new Date(data.fetchedAtMs).toLocaleTimeString()}`;
  if(!m){next.appendChild(el('p','Map: none'));root.replaceChildren(next);restoreUi();return}
  const activeGraphKeys=new Set(['graph:'+m.id]);
  Array.from(ui.graph.keys()).forEach(k=>{if(!activeGraphKeys.has(k))ui.graph.delete(k)});
  {
    const rootNode=m.nodes.find(n=>n.id===m.rootNodeId);
    next.appendChild(el('h2',`${rootNode?rootNode.goal:'TaskSpace'} (${m.id})`));
    const line=el('div');
    ['revision '+m.revision,'complete '+m.complete,'root '+m.rootNodeId,'finish '+m.finishNodeId,'current '+(m.currentNodeId||'none'),'ready '+m.readyNodeCount,'running '+m.runningNodeCount,'completed '+m.completedNodeCount].forEach(x=>line.appendChild(el('span',x,'pill')));
    next.appendChild(line);
    next.appendChild(renderGraph(m));
    next.appendChild(detail('nodes:'+m.id,'nodes',table(['id','role','status','goal','source refs','lease','results','events'],m.nodes.map(n=>[n.id,n.role,n.status,n.goal,list(n.sourceRefs),n.activeLease||'',list(n.resultIds),list(n.nodeEventIds)]))));
    if(m.edges.length){next.appendChild(detail('edges:'+m.id,'edges',table(['from','to'],m.edges.map(e=>[e.from,e.to]))))}
    if(m.leases.length){next.appendChild(detail('leases:'+m.id,'leases',table(['id','node','agent thread','agent path'],m.leases.map(l=>[l.id,l.nodeId,l.agentThreadId||'',l.agentPath||'']))))}
    if(m.results.length){
      next.appendChild(el('h3','results'));
      m.results.forEach(r=>{
        next.appendChild(detail('result:'+r.id,`${r.id} | node ${r.nodeId} | ${r.kind} | ${new Date(Number(r.createdAtMs)).toLocaleTimeString()}`,renderResultDetail(r)));
      });
    }
    if(m.nodeEvents.length){next.appendChild(detail('events:'+m.id,'node events',table(['id','node','kind','source','action','success','source event','raw ref','artifacts'],m.nodeEvents.map(e=>[e.id,e.nodeId,e.eventKind,e.source,e.actionClass||'',e.toolSuccess===null?'':String(e.toolSuccess),e.sourceEventId||'',e.rawRef||'',list(e.artifactRefs)]))))}
  }
  root.replaceChildren(next);
  restoreUi();
}
async function refresh(){
  try{renderIfChanged(await (await fetch('/snapshot.json',{cache:'no-store'})).json(),false)}
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
        assert!(ACTION_MAP_VIEWER_HTML.contains("className='graph'"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("activeSelection()"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("renderIfChanged"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("wheel to zoom"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("pointerdown"));
        assert!(
            ACTION_MAP_VIEWER_HTML
                .contains("document.createElementNS('http://www.w3.org/2000/svg'")
        );
        assert!(ACTION_MAP_VIEWER_HTML.contains("details[data-key]"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("restoreUi()"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("s.schemaVersion"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("s.map"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("m.rootNodeId"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("m.finishNodeId"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("m.revision"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("n.role"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("n.goal"));
        assert!(ACTION_MAP_VIEWER_HTML.contains("m.nodeEvents"));
        assert!(!ACTION_MAP_VIEWER_HTML.contains("canonicalKind"));
        assert!(!ACTION_MAP_VIEWER_HTML.contains("activeMapId"));
        assert!(!ACTION_MAP_VIEWER_HTML.contains("s.maps"));
        assert!(!ACTION_MAP_VIEWER_HTML.contains("cognitive state"));
    }
}
