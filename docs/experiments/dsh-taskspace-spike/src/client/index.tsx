/**
 * Browser half of the TaskMap spike: a right-Sidebar tab type whose body
 * renders the host-computed `whaleTaskspace` projection.
 *
 * This is the GUI path an installed bundle would build to `lib/client.js`
 * (`tsdown`), declared in the package manifest as:
 *
 * ```json
 * {
 *   "exports": { "./client": { "default": "./lib/client.js" } },
 *   "dsh": { "client": { "platform": "web", "inject": ["@deepseek-ai/dsh-client-ui-renderer"] } }
 * }
 * ```
 *
 * The host's client-module scan composes the entry into `window.__DSH_BOOT__`
 * and serves it under `/plugins`, so no frontend rebuild is involved. The data
 * path is the same one `ui-goal` uses: the host projection's `wire` value
 * arrives in the pushed `session/projection` frame and is read with
 * `useProjection('whaleTaskspace')` — no generated Remote client is required.
 * @module @deepseek-ai/dsh-whale-taskspace-spike/client
 */

import type { Context as ClientContext } from '@deepseek-ai/cordis'
import type {} from '@deepseek-ai/dsh-client-ui-renderer/client'
import type {} from '@deepseek-ai/dsh-client-ui-session/client'
import type {} from '@deepseek-ai/dsh-client-ui-sidebar-right/client'
import type { PropsRuntime } from '@deepseek-ai/dsh-client-ui-slots'
import type { TaskspaceView } from '../records.ts'

/** This implementation's identity; also the key of its body and title seats. */
const TYPE_ID = '@whale/taskmap'
/** Type discriminator the Sidebar opens by kind and routes addresses to. */
const KIND = 'taskmap'

/** Required browser services: the tab registry and the slot registry. */
export const inject = ['slots', 'sidebarRightTabs']

/**
 * Client plugin body: register the tab type, its body, and its chip title.
 * @param ctx - client root context carrying the registries.
 */
export function apply(ctx: ClientContext): void {
  ctx.effect(() => ctx.sidebarRightTabs.register({
    id: TYPE_ID,
    kind: KIND,
    patterns: ['dsh-resource://taskmap/**'],
    // An extension band outranks every shipped viewer for the same address.
    priority: 'extension',
    title: () => 'TaskMap',
    guide: [{ id: 'taskmap', order: 50, title: () => 'TaskMap', description: () => 'Canonical work map for this session' }],
  }), 'taskmap: tab type')

  ctx.effect(() => ctx.slots.inject('sidebar.right.pane.tab', () => ctx.slots.register({
    name: 'sidebar.right.pane.tab',
    id: TYPE_ID,
  }, TaskMapBody)), 'taskmap: tab body')

  ctx.effect(() => ctx.slots.inject('sidebar.right.pane.tab.title', () => ctx.slots.register({
    name: 'sidebar.right.pane.tab.title',
    id: TYPE_ID,
  }, () => <span>TaskMap</span>)), 'taskmap: tab title')
}

/**
 * The tab body: the whole map value comes from the host projection, so the
 * component holds no TaskMap state and issues no request of its own.
 */
function TaskMapBody({ useProjection }: PropsRuntime<'sidebar.right.pane.tab'>) {
  const view = useProjection('whaleTaskspace') as TaskspaceView | undefined
  if (view === undefined || view.mapId === null) return <p className="taskmap-empty">No active TaskMap.</p>
  return (
    <section className="taskmap">
      <header>
        {view.mapId} · rev {view.revision}{view.terminal ? ' · terminal' : ''}
      </header>
      <p className="taskmap-counts">
        {view.counts.done}/{view.nodes.length} done · {view.counts.ready} ready · {view.counts.blocked} blocked
      </p>
      <ul>
        {view.nodes.map(node => (
          <li key={node.id} data-status={node.status}>
            <code>{node.id}</code> [{node.status}] {node.title}
          </li>
        ))}
      </ul>
    </section>
  )
}
