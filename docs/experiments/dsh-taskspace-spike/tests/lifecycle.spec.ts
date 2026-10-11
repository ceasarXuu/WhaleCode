/**
 * Wave-2 lifecycle verification: the map survives a process restart
 * (JSONL resume) and coordinates a real in-process subagent — the lead stays
 * the only writer, while the child's result arrives as model context.
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
import { CONTROL_TOOL } from '../src/index.ts'
import type { ExecuteManifest, InitPayload } from '../src/index.ts'
import { harness, multiToolCallResponse, requestText, send, toolResults, waitForIdle } from './harness.ts'

const roots: string[] = []
const contexts: Context[] = []
afterEach(async () => {
  for (const ctx of contexts.splice(0).reverse()) await ctx.fiber.dispose()
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true })
})

function controlCall(id: string, action: string, payload?: object): { id: string; name: string; args: object } {
  return { id, name: CONTROL_TOOL, args: payload === undefined ? { action } : { action, manifest: JSON.stringify(payload) } }
}

describe('map continuity across a process restart', () => {
  it('resumes the canonical map and keeps committing after flush + reopen', async () => {
    const root = mkdtempSync(join(tmpdir(), 'dsh-map-resume-'))
    roots.push(root)
    const first = new MockAdapter([
      multiToolCallResponse([controlCall('i1', 'initialize', { mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })]),
      multiToolCallResponse([controlCall('e1', 'execute', { mapId: 'map-1', actions: [], mutations: [{ kind: 'add_work_nodes', nodes: [{ id: 'n2', title: 'second', blockedBy: ['n1'] }] }] })]),
      textResponse('first process done'),
    ])
    const a = await harness(first, root)
    contexts.push(a.ctx)
    const agent = await a.ctx.agentLoop.create(SessionId('resume-map'), { provider: 'mock', model: 'mock' })
    a.controller.setMode(agent.session, 'taskspace')
    a.controller.setProjection(agent.session, { projection: 'on-request' })
    send(agent, 'one')
    await waitForIdle(a.ctx, agent)
    expect(a.controller.stateOf(agent.session)?.map?.revision).toBe(2)
    await a.ctx.sessions.flush(agent.session)
    await a.ctx.fiber.dispose()
    contexts.splice(contexts.indexOf(a.ctx), 1)

    const second = new MockAdapter([
      multiToolCallResponse([controlCall('e2', 'execute', { mapId: 'map-1', expectedRevision: 2, actions: [], mutations: [{ kind: 'complete_node', nodeId: 'n1' }, { kind: 'activate_node', nodeId: 'n2' }] })]),
      textResponse('second process done'),
    ])
    const b = await harness(second, root)
    contexts.push(b.ctx)
    const handle = await b.ctx.agents.resume({
      resumeSessionId: agent.session.header.id,
      agentOptions: { provider: 'mock', model: 'mock' },
    })
    const resumed = handle.agent

    const restored = b.controller.stateOf(resumed.session)
    expect(restored?.map?.revision).toBe(2)
    expect(restored?.mode).toBe('taskspace')
    expect(restored?.policy.projection).toBe('on-request')

    send(resumed, 'two')
    await waitForIdle(b.ctx, resumed)
    const map = b.controller.stateOf(resumed.session)?.map
    expect(map?.revision).toBe(3)
    expect(map?.nodes.map(node => node.status)).toEqual(['done', 'active'])
    // Mode survived the restart: the resumed request still exposes the control tool.
    expect((second.requests[0]?.tools ?? []).some(tool => tool.name === CONTROL_TOOL)).toBe(true)
  })
})

describe('multi-agent TaskMap coordination', () => {
  it('lets a spawned child work while the lead owns the only map', async () => {
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
      multiToolCallResponse([controlCall('i1', 'initialize', { mapId: 'map-1', nodes: [{ id: 'n1', title: 'first' }] })]),
      textResponse('map ready'),
      textResponse('child finished n1'),
      multiToolCallResponse([controlCall('e1', 'execute', { mapId: 'map-1', expectedRevision: 1, actions: [], mutations: [{ kind: 'complete_node', nodeId: 'n1' }] })]),
      textResponse('lead closed the node'),
    ])
    ctx.llm.registerAdapter(['mock'], adapter)
    const lead = await ctx.agentLoop.create(SessionId('lead'), { provider: 'mock', model: 'mock' })
    const controller = ctx.whaleTaskspaceSpike
    controller.setMode(lead.session, 'taskspace')
    controller.setProjection(lead.session, { projection: 'on-request' })

    send(lead, 'start the map')
    await waitForIdle(ctx, lead)
    expect(controller.stateOf(lead.session)?.map?.revision).toBe(1)

    const activation = await ctx.subagents.startActivation({
      provider: 'spawn',
      label: 'work node n1',
      delivery: 'parent',
      signal: new AbortController().signal,
      request: { parent: lead, prompt: [{ type: 'text', text: 'Complete node n1 and report.' }] },
    })
    const child = ctx.agents.get(activation.childId) as Agent | undefined
    await activation.result
    await ctx.subagents.waitForChildren(lead)
    await lead.whenIdle()

    // The lead remains the only map writer: the child's fresh session carries
    // no TaskMap records at all.
    expect(child).toBeDefined()
    expect(child!.session.snapshotEvents().filter(event => pluginRecordOf(event) !== undefined)).toHaveLength(0)
    expect(controller.stateOf(child!.session)?.map).toBeNull()

    // The child's closing answer reached the lead as model context, and the
    // lead's control commit closed the node.
    expect(requestText(adapter.requests[3])).toContain('child finished n1')
    const map = controller.stateOf(lead.session)?.map
    expect(map?.revision).toBe(2)
    expect(map?.nodes[0]?.status).toBe('done')
    expect(toolResults(lead).some(result => result.text.includes('"operation":"execute"'))).toBe(true)
  })
})
