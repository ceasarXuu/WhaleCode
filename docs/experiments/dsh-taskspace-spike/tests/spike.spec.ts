/**
 * Wave-1 verification (kept as regression): the three suspected TaskSpace
 * blockers, probed with the real agent loop, real tool pipeline, real session
 * log and real projection registry — the only mock is the LLM adapter.
 *
 * Nothing in this file (or the package it imports) edits an upstream DSH file.
 */

import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { Context } from '@deepseek-ai/cordis'
import { SessionId, pluginRecordOf } from '@deepseek-ai/dsh-session'
import type { SessionEvent } from '@deepseek-ai/dsh-session'
import SessionStore from '@deepseek-ai/dsh-session'
import JsonlSessionPersistence from '@deepseek-ai/dsh-session-persistence-jsonl'
import { KNOWN_SESSION_EVENT_TYPES } from '../../../core/session/src/known-event-types.ts'
import { textResponse } from '../../../core/agent-loop/tests/mock-adapter.ts'
import { MockAdapter } from '../../../core/agent-loop/tests/mock-adapter.ts'
import { ACTIONS_RECORD, CONTROL_TOOL, MAP_RECORD, MODE_RECORD, taskspaceProjection } from '../src/index.ts'
import type { Agent } from '@deepseek-ai/dsh-agent'
import {
  events,
  harness,
  multiToolCallResponse,
  send,
  toolResults,
  waitForIdle,
} from './harness.ts'

const dirs: string[] = []
afterEach(async () => { for (const dir of dirs.splice(0)) await rm(dir, { recursive: true, force: true }) })

function manifestOf(overrides: object = {}): object {
  return { mapId: 'map-1', actions: [{ tool: 'probe' }], mutations: [], ...overrides }
}

function recordsOf(agent: Agent): readonly SessionEvent[] {
  return events(agent).filter(event => pluginRecordOf(event) !== undefined)
}

describe('blocker 1: batch-atomic admission without a core hook', () => {
  it('zero-dispatches every sibling when the control call is not first', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse([
        { id: 'c1', name: 'probe', args: { label: 'must-not-run' } },
        { id: 'c2', name: CONTROL_TOOL, args: { action: 'execute', manifest: JSON.stringify(manifestOf()) } },
      ]),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('batch-invalid'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')

    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.probeRuns).toBe(0)
    const results = toolResults(agent)
    expect(results).toHaveLength(2)
    expect(results.every(result => result.isError)).toBe(true)
    expect(results.every(result => result.text.includes('must be the first call'))).toBe(true)
  })

  it('zero-dispatches every sibling when the manifest does not match', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse([
        { id: 'c1', name: CONTROL_TOOL, args: { action: 'execute', manifest: JSON.stringify(manifestOf({ mutations: [] })) } },
        { id: 'c2', name: 'probe', args: { label: 'must-not-run' } },
        { id: 'c3', name: 'probe', args: { label: 'must-not-run-either' } },
      ]),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('batch-mismatch'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.probeRuns).toBe(0)
    const results = toolResults(agent)
    expect(results).toHaveLength(3)
    expect(results.every(result => result.isError)).toBe(true)
    expect(results.every(result => result.text.includes('manifest declares 1 action(s)'))).toBe(true)
    expect(controller.stateOf(agent.session)?.map).toBeNull()
  })

  it('zero-dispatches parallel siblings that the scheduler could start before the control call', async () => {
    // `probe` opts into parallel scheduling and is listed BEFORE the control
    // call, so a rolling scheduler would start it while the control call is
    // still in flight. The verdict is read from the committed assistant
    // message, so every sibling is denied at prepare time instead.
    const adapter = new MockAdapter([
      multiToolCallResponse([
        { id: 'p1', name: 'probe', args: { label: 'must-not-run' } },
        { id: 'p2', name: 'probe', args: { label: 'must-not-run-either' } },
        { id: 'c1', name: CONTROL_TOOL, args: { action: 'execute', manifest: JSON.stringify(manifestOf({ mutations: [] })) } },
      ]),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('batch-parallel'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.probeRuns).toBe(0)
    const results = toolResults(agent)
    expect(results).toHaveLength(3)
    expect(results.every(result => result.isError)).toBe(true)
  })

  it('rejects read_map when it shares the response with a sibling', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse([
        { id: 'c1', name: CONTROL_TOOL, args: { action: 'read_map' } },
        { id: 'p1', name: 'probe', args: { label: 'must-not-run' } },
      ]),
      textResponse('closed'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('batch-readmap'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.probeRuns).toBe(0)
    const results = toolResults(agent)
    expect(results).toHaveLength(2)
    expect(results.every(result => result.text.includes('only call'))).toBe(true)
  })

  it('commits the whole batch when the manifest matches, and hides nothing else', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse([
        { id: 'i1', name: CONTROL_TOOL, args: { action: 'initialize', manifest: JSON.stringify({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] }) } },
      ]),
      multiToolCallResponse([
        { id: 'e1', name: CONTROL_TOOL, args: { action: 'execute', manifest: JSON.stringify(manifestOf({ mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })) } },
        { id: 'e2', name: 'probe', args: { label: 'ran' } },
      ]),
      textResponse('done'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('batch-valid'), { provider: 'mock', model: 'mock' })

    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    expect(controller.probeRuns).toBe(1)
    const state = controller.stateOf(agent.session)
    expect(state?.map?.revision).toBe(2)
    expect(state?.map?.nodes[0]?.status).toBe('done')
    expect(toolResults(agent).some(result => !result.isError && result.text.includes('"operation":"execute"'))).toBe(true)
  })
})

describe('blocker 2: live standard <-> taskspace mode switch', () => {
  it('isolates mode per thread inside one process', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse([{ id: 'b1', name: CONTROL_TOOL, args: { action: 'read_map' } }]),
      textResponse('b-done'),
      multiToolCallResponse([{ id: 'a1', name: CONTROL_TOOL, args: { action: 'read_map' } }]),
      textResponse('a-done'),
    ])
    const { ctx, controller } = await harness(adapter)
    const standard = await ctx.agentLoop.create(SessionId('thread-standard'), { provider: 'mock', model: 'mock' })
    const taskspace = await ctx.agentLoop.create(SessionId('thread-taskspace'), { provider: 'mock', model: 'mock' })
    controller.setMode(taskspace.session, 'taskspace')

    send(standard, 'go')
    await waitForIdle(ctx, standard)
    send(taskspace, 'go')
    await waitForIdle(ctx, taskspace)

    // Request order: [0] standard step 1, [1] standard continuation after the
    // denial, [2] taskspace step 1, [3] taskspace continuation.
    const hasControl = (index: number): boolean =>
      (adapter.requests[index]?.tools ?? []).some(tool => tool.name === CONTROL_TOOL)
    expect(hasControl(0)).toBe(false)
    expect(hasControl(2)).toBe(true)

    const standardResult = toolResults(standard)
    expect(standardResult[0]?.text.includes('unavailable in standard mode')).toBe(true)
    const taskspaceResult = toolResults(taskspace)
    expect(taskspaceResult[0]?.text.includes('no canonical map is active')).toBe(true)
  })

  it('gates tool visibility per request and denies direct calls in standard mode', async () => {
    const adapter = new MockAdapter([
      textResponse('one'),
      textResponse('two'),
      multiToolCallResponse([{ id: 'c1', name: CONTROL_TOOL, args: { action: 'read_map' } }]),
      textResponse('three'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('mode-switch'), { provider: 'mock', model: 'mock' })

    const visible = (requestIndex: number): boolean =>
      (adapter.requests[requestIndex]?.tools ?? []).some(tool => tool.name === CONTROL_TOOL)

    send(agent, 'one')
    await waitForIdle(ctx, agent)
    expect(visible(0)).toBe(false)

    controller.setMode(agent.session, 'taskspace')
    send(agent, 'two')
    await waitForIdle(ctx, agent)
    expect(visible(1)).toBe(true)

    controller.setMode(agent.session, 'standard')
    send(agent, 'three')
    await waitForIdle(ctx, agent)
    expect(visible(2)).toBe(false)
    const denied = toolResults(agent).filter(result => result.text.includes('unavailable in standard mode'))
    expect(denied).toHaveLength(1)
    expect(denied[0]?.isError).toBe(true)
  })
})

describe('blocker 3: durable canonical state as an out-of-repo plugin', () => {
  it('writes ignorable plugin records, projects them, and replays them without the plugin', async () => {
    const root = await mkdtemp(join(tmpdir(), 'dsh-spike-'))
    dirs.push(root)
    const adapter = new MockAdapter([
      multiToolCallResponse([
        { id: 'i1', name: CONTROL_TOOL, args: { action: 'initialize', manifest: JSON.stringify({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] }) } },
      ]),
      multiToolCallResponse([
        { id: 'e1', name: CONTROL_TOOL, args: { action: 'execute', manifest: JSON.stringify(manifestOf({ mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })) } },
        { id: 'e2', name: 'probe', args: { label: 'ran' } },
      ]),
      textResponse('done'),
    ])
    const { ctx, controller } = await harness(adapter, root)
    const agent = await ctx.agentLoop.create(SessionId('durable'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'go')
    await waitForIdle(ctx, agent)

    const live = controller.stateOf(agent.session)
    expect(live?.map?.revision).toBe(2)
    expect(live?.mode).toBe('taskspace')

    const records = recordsOf(agent)
    expect(records.map(event => event.type)).toEqual([MODE_RECORD, MAP_RECORD, MAP_RECORD, ACTIONS_RECORD, ACTIONS_RECORD])
    expect(records.every(event => event.ignorable === true)).toBe(true)
    expect(KNOWN_SESSION_EVENT_TYPES.has(MAP_RECORD)).toBe(false)
    expect(live?.actions).toHaveLength(1)
    expect(live?.actions[0]?.state).toBe('released')

    await ctx.sessions.flush(agent.session)
    await ctx.fiber.dispose()

    const reader = new Context()
    await reader.plugin(SessionStore)
    await reader.plugin(JsonlSessionPersistence, { root })
    const handle = await reader.sessionPersistence.open(agent.session.header.id, 'read')
    const restored = await handle.read()
    await handle.close()

    const detached = reader.sessions.prepare(agent.session.header.id)
    const folded = restored.events.reduce(
      (state, event) => taskspaceProjection.apply(state, event),
      taskspaceProjection.init(detached.header, 0),
    )
    expect(folded).toEqual(live)
    await reader.fiber.dispose()
  })

  it('keeps plain responses queued when an ordinary tool batch has no control call', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse([{ id: 'p1', name: 'probe', args: { label: 'ordinary' } }]),
      textResponse('done'),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('ordinary'), { provider: 'mock', model: 'mock' })
    send(agent, 'go')
    await waitForIdle(ctx, agent)
    expect(controller.probeRuns).toBe(1)
    expect(toolResults(agent).every(result => !result.isError)).toBe(true)
  })
})

describe('spike hygiene: the harness this plugin runs in is unmodified upstream', () => {
  it('registers the spike tools on an ordinary harness and nothing else', async () => {
    const { ctx } = await harness(new MockAdapter([]))
    expect(ctx.tools.get('probe')?.name).toBe('probe')
    expect(ctx.tools.get(CONTROL_TOOL)?.name).toBe(CONTROL_TOOL)
    await ctx.fiber.dispose()
  })
})
