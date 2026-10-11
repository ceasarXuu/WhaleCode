/**
 * Feasibility probes for the read/exposure side and lineage:
 *
 * 1. RPC — an out-of-tree plugin exposes its map through Typert source-mode
 *    discovery without a generated descriptor or upstream registration;
 * 2. live viewer updates — the projection change feed reports the map unit;
 * 3. fork lineage — a forked child inherits the map and branches its own
 *    lineage without touching the parent's authority.
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
import * as Fork from '@deepseek-ai/dsh-subagent-fork-in-process'
import CommandRuntime from '@deepseek-ai/dsh-commands'
import TypertRegistry from '@deepseek-ai/dsh-typert-registry'
import TypertGatewayService from '@deepseek-ai/dsh-api-gateway'
import { MockAdapter, textResponse } from '../../../core/agent-loop/tests/mock-adapter.ts'
import SubagentRuntime from '../../../subagent/subagent/src/index.ts'
import { mountLocalActivations } from '../../../subagent/subagent/tests/local-activation.ts'
import { mountWorkingDirectoryFixture } from '../../../subagent/subagent/tests/working-directory-fixture.ts'
import { WhaleTaskspaceSpike, taskspaceCommands } from '../src/index.ts'
import { CONTROL_TOOL, MAP_RECORD } from '../src/index.ts'
import type { InitPayload } from '../src/index.ts'
import { harness, multiToolCallResponse, send, waitForIdle } from './harness.ts'

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

describe('external exposure: RPC and viewer change feed', () => {
  it('serves the map over Typert source-mode discovery and notifies projection listeners', async () => {
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', goal: 'expose', nodes: [{ id: 'n1', title: 'first' }] })),
      textResponse('ready'),
      multiToolCallResponse([controlCall('e1', 'execute', { mapId: 'map-1', expectedRevision: 1, actions: [], mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })]),
      textResponse('done'),
    ])
    const { ctx, controller } = await harness(adapter)
    await ctx.plugin(TypertRegistry)
    await ctx.plugin(TypertGatewayService)
    const agent = await ctx.agentLoop.create(SessionId('exposed'), { provider: 'mock', model: 'mock' })
    controller.setMode(agent.session, 'taskspace')
    send(agent, 'one')
    await waitForIdle(ctx, agent)
    expect(controller.stateOf(agent.session)?.map?.revision).toBe(1)

    // The gateway discovers the service by its runtime `typertRemote` binding
    // and the `@Remote` marker; no generated descriptor is registered.
    const view = await ctx.typertGateway.invoke({
      namespace: 'whaleTaskspaceSpike',
      method: 'get',
      args: { sessionId: String(agent.session.header.id) },
      signal: new AbortController().signal,
    })
    expect(view).toMatchObject({ mapId: 'map-1', revision: 1, goal: 'expose', terminal: false })
    const unknown = await ctx.typertGateway.invoke({
      namespace: 'whaleTaskspaceSpike',
      method: 'get',
      args: { sessionId: 'missing' },
      signal: new AbortController().signal,
    })
    expect(unknown).toBeNull()

    // User commands are ordinary plugin registrations: the demo's
    // /taskspace and /map-projection surface needs no core support.
    await ctx.plugin(taskspaceCommands)
    const signal = new AbortController().signal
    expect((await ctx.commands.execute(agent, '/taskspace off', [], signal))?.result)
      .toEqual({ kind: 'success', text: 'TaskMap mode off.' })
    expect(controller.stateOf(agent.session)?.mode).toBe('standard')
    expect((await ctx.commands.execute(agent, '/map-projection always', [], signal))?.result)
      .toEqual({ kind: 'success', text: 'TaskMap projection: always' })
    expect(controller.stateOf(agent.session)?.policy.projection).toBe('always')
    expect((await ctx.commands.execute(agent, '/taskspace', [], signal))?.result)
      .toEqual({ kind: 'success', text: 'TaskMap mode on.' })
    expect(ctx.commands.list(agent).map(item => item.name)).toEqual(['map-projection', 'taskspace'])

    // A viewer subscribes to the projection change feed and sees the map unit
    // change when the next commit lands.
    const changed = Promise.withResolvers<string>()
    const dispose = ctx.sessionProjections.onChanged((session, key) => {
      if (session !== agent.session || key !== 'whaleTaskspace') return
      changed.resolve(key)
    })
    send(agent, 'two')
    await waitForIdle(ctx, agent)
    dispose()
    expect(controller.stateOf(agent.session)?.map?.revision).toBe(2)
    await expect(changed.promise).resolves.toBe('whaleTaskspace')
  })
})

describe('fork lineage: a forked child inherits and branches the map', () => {
  it('inherits the parent map prefix and keeps its own later commits off the parent', async () => {
    const ctx = new Context()
    contexts.push(ctx)
    await mountLocalActivations(ctx)
    await mountAgentLoopTestDependencies(ctx)
    await ctx.plugin(WhaleTaskspaceSpike)
    await ctx.plugin(AgentLoop, { agents: [] })
    await mountWorkingDirectoryFixture(ctx)
    await ctx.plugin(SubagentRuntime)
    await ctx.plugin(Fork, { providerName: 'fork' })
    const adapter = new MockAdapter([
      multiToolCallResponse(init({ mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })),
      textResponse('parent ready'),
      multiToolCallResponse([controlCall('fe1', 'execute', { mapId: 'map-1', expectedRevision: 1, actions: [], mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })]),
      textResponse('fork done'),
    ])
    ctx.llm.registerAdapter(['mock'], adapter)
    const parent = await ctx.agentLoop.create(SessionId('fork-parent'), { provider: 'mock', model: 'mock' })
    const controller = ctx.whaleTaskspaceSpike
    controller.setMode(parent.session, 'taskspace')
    controller.setProjection(parent.session, { projection: 'on-request' })
    send(parent, 'start')
    await waitForIdle(ctx, parent)
    expect(controller.stateOf(parent.session)?.map?.revision).toBe(1)

    const activation = await ctx.subagents.startActivation({
      provider: 'fork',
      label: 'continue in a fork',
      delivery: 'caller',
      signal: new AbortController().signal,
      request: { parent, prompt: [{ type: 'text', text: 'Complete node n1 in your fork.' }] },
    })
    const child = ctx.agents.get(activation.childId) as Agent | undefined
    await activation.result

    // The fork inherits the parent's map prefix (including its plan and mode),
    // and its own execute branches a new lineage revision.
    expect(child).toBeDefined()
    const childState = controller.stateOf(child!.session)
    expect(childState?.map?.mapId).toBe('map-1')
    expect(childState?.map?.revision).toBe(2)
    expect(childState?.map?.nodes[0]?.status).toBe('done')
    expect(childState?.mode).toBe('taskspace')

    // The parent stays at its own revision: the fork's commit never landed in
    // the parent's log.
    expect(controller.stateOf(parent.session)?.map?.revision).toBe(1)
    expect(controller.stateOf(parent.session)?.map?.nodes[0]?.status).toBe('pending')
    const parentRecords = parent.session.snapshotEvents().filter(event => pluginRecordOf(event)?.type === MAP_RECORD)
    expect(parentRecords).toHaveLength(1)
    const childOwnRecords = child!.session.snapshotEvents().filter(event => pluginRecordOf(event)?.type === MAP_RECORD)
    expect(childOwnRecords.length).toBeGreaterThanOrEqual(2)
  })
})
