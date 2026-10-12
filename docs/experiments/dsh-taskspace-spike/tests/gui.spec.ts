/**
 * GUI-path feasibility, headless:
 *
 * 1. an installed package that declares `dsh.client` joins the Web boot graph
 *    at runtime (no frontend rebuild, no upstream edit), and the host serves
 *    its bundle under `/plugins`;
 * 2. an out-of-tree tab type registers into the right Sidebar and outranks a
 *    shipped viewer for its address.
 *
 * The rendered component itself is the same projection read `ui-goal` uses
 * (`useProjection('whaleTaskspace')`); the host side of that read is covered
 * by `exposure.spec.ts` (wire view + change feed).
 */

import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { afterEach, describe, expect, it } from 'vitest'
import { Context } from '@deepseek-ai/cordis'
import type { WebRoute, WebServer } from '@deepseek-ai/dsh-host-webserver'
import { ClientModuleRegistry } from '../../../client/modules/src/index.ts'
import { SidebarRightTabRegistry } from '../../../client/ui-sidebar-right/src/client/tab-registry.ts'

const contexts: Context[] = []
const roots: string[] = []
afterEach(async () => {
  for (const ctx of contexts.splice(0).reverse()) await ctx.fiber.dispose()
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true })
})

/** Write one installed package whose built client bundle exists on disk. */
function writeClientPackage(packageName: string): { root: string; clientPath: string } {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'dsh-taskmap-gui-')))
  roots.push(root)
  const pkgRoot = join(root, 'node_modules', ...packageName.split('/'))
  const clientPath = join(pkgRoot, 'lib', 'client.js')
  mkdirSync(dirname(clientPath), { recursive: true })
  writeFileSync(join(pkgRoot, 'package.json'), JSON.stringify({
    name: packageName,
    exports: { './client': './lib/client.js', './package.json': './package.json' },
    dsh: { client: { platform: 'web' } },
  }))
  writeFileSync(clientPath, 'globalThis.__TASKMAP_CLIENT__ = { kind: "taskmap" }\n')
  return { root, clientPath }
}

/** Invoke one registered `/plugins` route and capture its response. */
async function requestRoute(route: WebRoute, url: string): Promise<{ status: number; body: string }> {
  let status = 0
  let body = ''
  const response = {
    writeHead(nextStatus: number) {
      status = nextStatus
      return response
    },
    end(chunk?: Uint8Array) {
      body = chunk === undefined ? '' : Buffer.from(chunk).toString('utf8')
      return response
    },
  } as unknown as ServerResponse
  await route.handler({ method: 'GET', url } as IncomingMessage, response)
  return { status, body }
}

describe('GUI activation path for an installed TaskMap bundle', () => {
  it('composes the installed client face into the Web boot graph and serves its bundle', async () => {
    const packageName = '@whale/dsh-taskmap'
    const { root, clientPath } = writeClientPackage(packageName)
    const ctx = new Context()
    contexts.push(ctx)
    ctx.baseUrl = `${pathToFileURL(root).href}/`
    ctx.provide('loader', {
      *entries() {
        yield {
          options: { name: packageName },
          fiber: {},
          disabled: false,
          parent: { tree: { ctx: { baseUrl: ctx.baseUrl } } },
        }
      },
    })
    let route: WebRoute | undefined
    ctx.provide('webServer', {
      port: 0,
      register(candidate: WebRoute) {
        if (candidate.path === '/plugins') route = candidate
        return () => { route = undefined }
      },
      tapIndex: () => () => {},
    } as unknown as WebServer)
    const service = new ClientModuleRegistry(ctx)

    // The boot graph is what the host injects as window.__DSH_BOOT__: the
    // installed package is a first-class client entry with its own bundle URL.
    const graph = service.graph()
    const entry = graph.entries.find(candidate => candidate.id === packageName)
    expect(entry).toBeDefined()
    expect(entry?.url).toContain('/client.js')
    expect(graph.batches.flatMap(batch => batch.entries)).toContain(packageName)

    // The same service serves the bundle bytes under /plugins (the route
    // registration follows service activation, so poll for it).
    await expect.poll(() => route !== undefined).toBe(true)
    const served = await requestRoute(route!, entry!.url)
    expect(served.status).toBe(200)
    expect(served.body).toContain('__TASKMAP_CLIENT__')
    expect(clientPath.endsWith('client.js')).toBe(true)
  })

  it('lets an out-of-tree tab type claim its address ahead of a shipped viewer', async () => {
    const ctx = new Context()
    contexts.push(ctx)
    const tabs = new SidebarRightTabRegistry(ctx)
    tabs.register({
      id: '@deepseek-ai/dsh-client-ui-sidebar-files',
      kind: 'files',
      patterns: ['dsh-resource://**'],
      priority: 'builtin',
      title: () => 'Files',
    })
    tabs.register({
      id: '@whale/taskmap',
      kind: 'taskmap',
      patterns: ['dsh-resource://taskmap/**'],
      priority: 'extension',
      title: () => 'TaskMap',
      guide: [{ id: 'taskmap', order: 50, title: () => 'TaskMap' }],
    })

    const address = 'dsh-resource://taskmap/session/s-1'
    expect(tabs.candidates(address).map(candidate => candidate.id)[0]).toBe('@whale/taskmap')
    expect(tabs.claim(address)).toMatchObject({ kind: 'taskmap', title: 'TaskMap' })
    // Opening by kind works too: the guide box opens the contributing type.
    expect(tabs.get('taskmap')?.id).toBe('@whale/taskmap')
  })
})
