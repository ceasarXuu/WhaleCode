/**
 * Feasibility probes that decide whether the remaining TaskMap properties are
 * reachable from a pure plugin:
 *
 * 1. cross-session authority — a bound member session writes through the
 *    owner's canonical log, which stays the single map authority;
 * 2. reservation integrity under cancellation — an aborted sibling must
 *    release its reservation instead of leaking it.
 */

import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { Context } from '@deepseek-ai/cordis'
import { SessionId, pluginRecordOf } from '@deepseek-ai/dsh-session'
import type { Agent } from '@deepseek-ai/dsh-agent'
import AgentLoop from '@deepseek-ai/dsh-agent-loop'
import { mountAgentLoopTestDependencies } from '@deepseek-ai/dsh-agent-loop-testkit'
import * as Spawn from '@deepseek-ai/dsh-subagent-spawn-in-process'
import { MockAdapter, textResponse } from '../../../core/agent-loop/tests/mock-adapter.ts'
import SubagentRuntime from '../../../subagent/subagent/src/index.ts'
import { mountLocalActivations } from '../../../subagent/subagent/tests/local-activation.ts'
import { mountWorkingDirectoryFixture } from '../../../subagent/subagent/tests/working-directory-fixture.ts'
import { WhaleTaskspaceSpike } from '../src/index.ts'
import { CONTROL_TOOL, MAP_RECORD } from '../src/index.ts'
import type { ExecuteManifest, InitPayload } from '../src/index.ts'
import { harness, multiToolCallResponse, requestText, send, toolResults, waitForIdle } from './harness.ts'

const contexts: Context[] = []
const roots: string[] = []
afterEach(async () => {
  for (const ctx of contexts.splice(0).reverse()) await ctx.fiber.dispose()
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true })
})

function controlCall(id: string, action: string, payload?: object): { id: string; name: string; args: object } {
  return { id, name: CONTROL_TOOL, args: payload === undefined ? { action } : { action, manifest: JSON.stringify(payload) } }
}

function init(payload: InitPayload): object[] {
  return [controlCall('i1', 'initialize', payload)]
}

function probe(id: string, label: string, extra: object = {}): { id: string; name: string; args: object } {
  return { id, name: 'probe', args: { label, ...extra } }
}

describe('cross-session authority: a bound member writes through the owner log', () => {
  it('lets a delegated child commit to the lead-owned map without owning records', async () => {
    const ctx = new Context()
    contexts.push(ctx)
    await mountLocalActivations(ctx)
    await mountAgentLoopTestDependencies(ctx)
    await ctx.plugin(WhaleTaskspaceSpike)
    await ctx.plugin(AgentLoop, { agents: [] })
    await mountWorkingDirectoryFixture(ctx)
    await ctx.plugin(SubagentRuntime)
    await ctx.plugin(Spawn, { providerName: 'spawn' })
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      textResponse('map ready'),
      multiToolCallResponse([controlCall('cr1', 'read_map')]),
      multiToolCallResponse([controlCall('ce1', 'execute', { mapId: 'map-1', expectedRevision: 1, actions: [], mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })]),
      textResponse('child closed'),
      textResponse('lead notified'),
    ])
    ctx.llm.registerAdapter(['mock'], adapter)
    const lead = await ctx.agentLoop.create(SessionId('lead'), { provider: 'mock', model: 'mock' })
    const controller = ctx.whaleTaskspaceSpike
    controller.setMode(lead.session, 'taskspace')
    controller.setProjection(lead.session, { projection: 'on-request' })
    // Bind any delegated child to the lead's map as soon as it is created:
    // `agent/created` is awaited before queued work, so the child's first
    // request already carries taskspace mode and the control tool.
    ctx.on('agent/created', ({ agent }) => {
      if (agent.session.header.id === lead.session.header.id) return
      controller.bindTeam(agent.session, lead.session)
    })

    send(lead, 'start')
    await waitForIdle(ctx, lead)
    expect(controller.stateOf(lead.session)?.map?.revision).toBe(1)

    const activation = await ctx.subagents.startActivation({
      provider: 'spawn',
      label: 'complete node n1',
      delivery: 'parent',
      signal: new AbortController().signal,
      request: { parent: lead, prompt: [{ type: 'text', text: 'Read the map, then complete node n1.' }] },
    })
    const child = ctx.agents.get(activation.childId) as Agent | undefined
    await activation.result
    await ctx.subagents.waitForChildren(lead)
    await lead.whenIdle()

    // The owner log stays the single authority: the child's execute committed
    // revision 2 into the lead's log, while the child's own log carries only
    // the binding and its own tool traffic.
    const leadMapRecords = lead.session.snapshotEvents().filter(event => pluginRecordOf(event)?.type === MAP_RECORD)
    expect(leadMapRecords.map(event => (pluginRecordOf(event)?.data as { revision?: number }).revision)).toEqual([1, 2])
    expect(controller.stateOf(lead.session)?.map?.nodes[0]?.status).toBe('done')

    expect(child).toBeDefined()
    expect(controller.stateOf(child!.session)?.binding?.ownerSessionId).toBe(String(lead.session.header.id))
    expect(controller.stateOf(child!.session)?.binding?.mapId).toBe('map-1')
    const childMapRecords = child!.session.snapshotEvents().filter(event => pluginRecordOf(event)?.type === MAP_RECORD)
    expect(childMapRecords).toHaveLength(0)

    // Cross-session read works: the child's read_map returned the owner's map.
    const childRead = toolResults(child!).find(result => result.text.includes('"nodes"'))
    expect(childRead?.text).toContain('"mapId":"map-1"')
    expect(childRead?.text).toContain('"revision":1')
    const childExecute = toolResults(child!).find(result => result.text.includes('"operation":"execute"'))
    expect(childExecute?.text).toContain('"revision":2')

    // The child's closing answer reached the lead as model context.
    expect(requestText(adapter.requests.at(-1))).toBeDefined()
    expect(adapter.requests.some(request => requestText(request).includes('child closed'))).toBe(true)
  })
})

describe('reservation integrity under cancellation', () => {
  it('releases a deferred reservation when the sibling is cancelled instead of leaking it', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      textResponse('ready'),
      multiToolCallResponse([
        controlCall('e1', 'execute', {
          mapId: 'map-1',
          expectedRevision: 1,
          actions: [{ tool: 'probe', nodeId: 'n1' }],
          mutations: [{ kind: 'complete_node', nodeId: 'n1' }],
          deferred: [{ mutationIndex: 0, actionIndex: 0 }],
        } satisfies ExecuteManifest),
        probe('p1', 'will-hang', { hang: true }),
      ]),
    ])
    const { ctx, controller } = await harness(adapter)
    const agent = await ctx.agentLoop.create(SessionId('cancel'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'one')
    await waitForIdle(ctx, agent)
    expect(controller.stateOf(agent.session)?.map?.revision).toBe(1)

    const started = Promise.withResolvers<undefined>()
    ctx.on('tools/execute', async (exec, next) => {
      if (exec.callId === 'p1') started.resolve(undefined)
      return next()
    })
    send(agent, 'two')
    await started.promise
    agent.cancel({ kind: 'user' })
    await agent.whenIdle()

    const state = controller.stateOf(agent.session)
    expect(controller.probeRuns).toBe(1)
    expect(state?.actions.some(entry => entry.state === 'reserved')).toBe(false)
    expect(state?.actions.at(-1)?.state).toBe('failed')
    expect(state?.actions.at(-1)?.appliedMutation).toBeUndefined()
    // The deferred completion stayed unapplied: the map is still at revision 1.
    expect(state?.map?.revision).toBe(1)
    expect(state?.map?.nodes[0]?.status).toBe('pending')
  })
})
