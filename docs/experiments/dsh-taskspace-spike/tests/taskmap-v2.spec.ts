/**
 * Wave-2 verification: the TaskMap idea replicated as a plugin — work-node
 * DAG with a ready frontier, CAS + terminal gates, deferred mutations with
 * action reservations, context projection policies, and a client view.
 */

import { afterEach, describe, expect, it } from 'vitest'
import { SessionId } from '@deepseek-ai/dsh-session'
import { textResponse } from '../../../core/agent-loop/tests/mock-adapter.ts'
import { MockAdapter } from '../../../core/agent-loop/tests/mock-adapter.ts'
import { CONTROL_TOOL, frontierOf } from '../src/index.ts'
import type { ExecuteManifest, InitPayload } from '../src/index.ts'
import {
  events,
  harness,
  multiToolCallResponse,
  requestText,
  requestsContaining,
  send,
  toolResults,
  waitForIdle,
} from './harness.ts'

afterEach(() => { /* harness contexts are disposed by the tests */ })

function controlCall(id: string, action: string, payload?: object): { id: string; name: string; args: object } {
  return {
    id,
    name: CONTROL_TOOL,
    args: payload === undefined ? { action } : { action, manifest: JSON.stringify(payload) },
  }
}

function init(payload: InitPayload): object[] {
  return [controlCall('i1', 'initialize', payload)]
}

function execute(manifest: ExecuteManifest, siblings: readonly { id: string; name: string; args: object }[] = []): object[] {
  return [controlCall('e1', 'execute', manifest), ...siblings]
}

function probe(id: string, label: string, fail = false): { id: string; name: string; args: object } {
  return { id, name: 'probe', args: fail ? { label, fail: true } : { label } }
}

describe('TaskMap domain: DAG, frontier and invariants', () => {
  it('initializes a dependency graph and derives the ready frontier', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({
        mapId: 'map-1',
        goal: 'ship the map',
        nodes: [
          { id: 'n1', title: 'first' },
          { id: 'n2', title: 'second', blockedBy: ['n1'] },
          { id: 'n3', title: 'third', blockedBy: ['n2'] },
        ],
      })),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'activate_node', nodeId: 'n1' }, { kind: 'complete_node', nodeId: 'n1' }] })),
      textResponse('done'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('dag'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const map = controller.stateOf(agent.session)?.map
    expect(map?.goal).toBe('ship the map')
    expect(map?.revision).toBe(2)
    expect(map?.nodes.map(node => node.status)).toEqual(['done', 'pending', 'pending'])
    expect(frontierOf(map!)).toEqual(['n2'])
  })

  it('rejects a dependency cycle without committing anything', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({
        mapId: 'map-1',
        nodes: [
          { id: 'n1', title: 'first', blockedBy: ['n2'] },
          { id: 'n2', title: 'second', blockedBy: ['n1'] },
        ],
      })),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('cycle'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.stateOf(agent.session)?.map).toBeNull()
    expect(toolResults(agent)[0]?.text.includes('dependency cycle')).toBe(true)
  })

  it('rejects a cycle introduced by set_blocked_by without a partial commit', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({
        mapId: 'map-1',
        nodes: [{ id: 'n1', title: 'first' }, { id: 'n2', title: 'second', blockedBy: ['n1'] }],
      })),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'set_blocked_by', nodeId: 'n1', blockedBy: ['n2'] }] })),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('cycle-edit'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const map = controller.stateOf(agent.session)?.map
    expect(map?.revision).toBe(1)
    expect(map?.nodes[0]?.blockedBy).toEqual([])
    expect(toolResults(agent).some(result => result.text.includes('dependency cycle'))).toBe(true)
  })
})

describe('TaskMap control plane: CAS and terminal gates', () => {
  it('rejects a stale expectedRevision for the whole batch with zero dispatch', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      multiToolCallResponse(execute({ mapId: 'map-1', expectedRevision: 99, actions: [{ tool: 'probe' }], mutations: [] }, [probe('p1', 'must-not-run')])),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('cas'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.probeRuns).toBe(0)
    const rejected = toolResults(agent).filter(result => result.text.includes('revision conflict'))
    expect(rejected).toHaveLength(2)
    expect(rejected.every(result => result.isError)).toBe(true)
    expect(rejected.every(result => result.text.includes('revision conflict: expected 99'))).toBe(true)
    expect(controller.stateOf(agent.session)?.map?.revision).toBe(1)
  })

  it('gates finish_map on unfinished nodes and terminal maps on execute', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      multiToolCallResponse([controlCall('f1', 'finish_map')]),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })),
      textResponse('checkpoint'),
      multiToolCallResponse([controlCall('f2', 'finish_map')]),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [{ tool: 'probe' }], mutations: [{ kind: 'add_work_nodes', nodes: [{ id: 'n2', title: 'second' }] }] }, [probe('p1', 'must-not-run')])),
      multiToolCallResponse([controlCall('r1', 'reopen_map')]),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'add_work_nodes', nodes: [{ id: 'n2', title: 'second' }] }, { kind: 'activate_node', nodeId: 'n2' }] })),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('gates'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')

    send(agent, 'go')
    await waitForIdle(ctx, agent)
    // finish with a pending node was denied before dispatch; one node completed.
    expect(controller.stateOf(agent.session)?.map?.revision).toBe(2)
    expect(toolResults(agent).some(result => result.text.includes('unfinished node(s): n1'))).toBe(true)

    send(agent, 'continue')
    await waitForIdle(ctx, agent)

    const map = controller.stateOf(agent.session)?.map
    // finish succeeded, the terminal map rejected the next mutating batch,
    // reopen lifted it, and the final execute activated the new node.
    expect(controller.probeRuns).toBe(0)
    expect(map?.terminal).toBe(false)
    expect(map?.revision).toBe(5)
    expect(map?.nodes.map(node => node.status)).toEqual(['done', 'active'])
    const texts = toolResults(agent).map(result => result.text)
    expect(texts.some(text => text.includes('is terminal; reopen before mutating'))).toBe(true)
  })
})

describe('TaskMap reservations: deterministic ids and deferred mutations', () => {
  it('keeps a deferred completion pending until its sibling succeeds', async () => {
    const manifest: ExecuteManifest = {
      mapId: 'map-1',
      actions: [{ tool: 'probe', nodeId: 'n1' }],
      mutations: [{ kind: 'complete_node', nodeId: 'n1' }],
      deferred: [{ mutationIndex: 0, actionIndex: 0 }],
    }
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      multiToolCallResponse(execute(manifest, [probe('p1', 'will-fail', true)])),
      multiToolCallResponse(execute(manifest, [probe('p2', 'will-succeed')])),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('deferred'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const state = controller.stateOf(agent.session)
    expect(controller.probeRuns).toBe(2)
    // Only the successful sibling applied the deferred mutation.
    expect(state?.map?.revision).toBe(2)
    expect(state?.map?.nodes[0]?.status).toBe('done')
    const [failed, released] = state?.actions.slice(-2) ?? []
    expect(failed?.id).toBe('action:map-1:e1:0')
    expect(failed?.callId).toBe('p1')
    expect(failed?.state).toBe('failed')
    expect(failed?.appliedMutation).toBeUndefined()
    expect(released?.id).toBe('action:map-1:e1:0')
    expect(released?.callId).toBe('p2')
    expect(released?.state).toBe('released')
    expect(released?.appliedMutation).toEqual({ kind: 'complete_node', nodeId: 'n1' })
  })
})

describe('TaskMap context projection policies', () => {
  it('renders guidance, appended changes, on-request silence and always-context', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      textResponse('ok1'),
      multiToolCallResponse(execute({ mapId: 'map-1', expectedRevision: 1, actions: [], mutations: [{ kind: 'add_work_nodes', nodes: [{ id: 'n2', title: 'second' }] }] })),
      textResponse('ok2'),
      textResponse('ok3'),
      textResponse('ok4'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('context'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')

    send(agent, 'one')
    await waitForIdle(ctx, agent)
    // Turn 1: guidance is in the very first request; the append policy injects
    // the committed map into the tool-continuation request.
    expect(requestText(adapter.requests[0])).toContain('TaskMap is active in this session.')
    expect(requestText(adapter.requests[0])).not.toContain('TaskMap "map-1"')
    expect(requestText(adapter.requests[1])).toContain('TaskMap "map-1" revision 1')

    controller.setProjection(agent.session, { projection: 'on-request' })
    send(agent, 'two')
    await waitForIdle(ctx, agent)
    // on-request: the committed revision 2 is not pushed into the next request.
    expect(requestsContaining(adapter.requests, 'TaskMap "map-1" revision 2')).toEqual([])

    controller.setProjection(agent.session, { projection: 'always' })
    send(agent, 'three')
    await waitForIdle(ctx, agent)
    // always: the dynamic context materializes the map even without a change.
    expect(requestText(adapter.requests[4])).toContain('TaskMap "map-1" revision 2')

    controller.setMode(agent.session, 'standard')
    send(agent, 'four')
    await waitForIdle(ctx, agent)
    expect(requestText(adapter.requests[5])).not.toContain('TaskMap is active in this session.')
  })

  it('exposes a bounded client view through the projection snapshot', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', goal: 'viewer', nodes: [{ id: 'n1', title: 'first' }, { id: 'n2', title: 'second', blockedBy: ['n1'] }] })),
      textResponse('ok'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('viewer'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const snapshot = ctx.sessionProjections.snapshot(agent.session, ['whaleTaskspace'])
    const view = snapshot.values.whaleTaskspace as {
      mapId: string
      revision: number
      counts: { ready: number, pending: number }
      nodes: readonly { id: string, status: string }[]
    }
    expect(view.mapId).toBe('map-1')
    expect(view.revision).toBe(1)
    expect(view.counts.ready).toBe(1)
    expect(view.nodes.map(node => node.id)).toEqual(['n1', 'n2'])
    expect(typeof snapshot.asOfSeq).toBe('number')
  })
})

describe('TaskMap error paths and uncovered control edges', () => {
  it('rejects an unknown control action for the whole batch', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      multiToolCallResponse([controlCall('x1', 'teleport'), probe('p1', 'must-not-run')]),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('unknown-action'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.probeRuns).toBe(0)
    expect(toolResults(agent).some(result => result.text.includes('unknown taskspace_control action'))).toBe(true)
  })

  it('read_map returns the canonical map and the action ledger', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      multiToolCallResponse([controlCall('r1', 'read_map')]),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('read-back'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const read = toolResults(agent).map(result => result.text).find(text => text.includes('"nodes"') && text.includes('"actions"'))
    expect(read).toBeDefined()
    const parsed = JSON.parse(read!) as { map: { mapId: string, revision: number }, actions: unknown[] }
    expect(parsed.map.mapId).toBe('map-1')
    expect(parsed.map.revision).toBe(1)
    expect(parsed.actions).toEqual([])
  })

  it('rejects duplicate node ids and duplicate add_work_nodes without a partial commit', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }, { id: 'n1', title: 'again' }] })),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('dupes'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)
    expect(controller.stateOf(agent.session)?.map).toBeNull()
    expect(toolResults(agent)[0]?.text.includes('appears twice')).toBe(true)

    const second = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'add_work_nodes', nodes: [{ id: 'n1', title: 'again' }] }] })),
      textResponse('closed'),
    ])
    const b = await harness(second)
    const other = await b.ctx.agentLoop.create(SessionId('dupes-add'), { provider: 'mock', model: 'mock' })
    b.controller.setMode(other.session, 'taskspace')
    send(other, 'go')
    await waitForIdle(b.ctx, other)
    expect(b.controller.stateOf(other.session)?.map?.revision).toBe(1)
    expect(toolResults(other).some(result => result.text.includes('already exists'))).toBe(true)
    await b.ctx.fiber.dispose()
  })

  it('rejects an unsupported mutation and an unmet activation without partial commits', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }, { id: 'n2', title: 'second', blockedBy: ['n1'] }] })),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'activate_node', nodeId: 'n2' }] })),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'unblock_node', nodeId: 'n2' }, { kind: 'block_node', nodeId: 'n2' }, { kind: 'unblock_node', nodeId: 'n2' }] })),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('activation'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(toolResults(agent).some(result => result.text.includes('is not ready'))).toBe(true)
    const map = controller.stateOf(agent.session)?.map
    expect(map?.revision).toBe(2)
    expect(map?.nodes[1]?.status).toBe('pending')
  })

  it('denies redundant finish and reopen batches', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      multiToolCallResponse(execute({ mapId: 'map-1', actions: [], mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })),
      multiToolCallResponse([controlCall('f1', 'finish_map')]),
      multiToolCallResponse([controlCall('f2', 'finish_map')]),
      multiToolCallResponse([controlCall('r1', 'reopen_map')]),
      multiToolCallResponse([controlCall('r2', 'reopen_map')]),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('redundant'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const texts = toolResults(agent).map(result => result.text)
    expect(texts.some(text => text.includes('is already terminal'))).toBe(true)
    expect(texts.some(text => text.includes('is not terminal'))).toBe(true)
    expect(controller.stateOf(agent.session)?.map?.revision).toBe(4)
  })
})

describe('TaskMap domain unit edges', () => {
  it('bounds the rendered summary and reports omitted nodes', async () => {
    const nodes = Array.from({ length: 40 }, (_value, index) => ({ id: `n${index}`, title: `node number ${index} with a fairly long title` }))
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-big', nodes })),
      textResponse('ok'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('bounded'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    controller.setProjection(agent.session, { projection: 'append', maxChars: 320 })
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const summary = controller.summaryOf(agent.session)
    expect(summary).toBeDefined()
    expect(summary!.length).toBeLessThanOrEqual(320)
    expect(summary).toContain('more node')
    expect(events(agent).length).toBeGreaterThan(0)
  })
})
