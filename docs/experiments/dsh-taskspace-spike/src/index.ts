/**
 * TaskMap spike plugin — a pure out-of-tree-style DSH plugin replicating the
 * WhaleCode TaskSpace idea on DSH seams:
 *
 * - canonical work-node DAG whose only write path is the `taskspace_control`
 *   tool, admitted as a whole batch (control-first manifest, zero dispatch on
 *   mismatch, CAS by revision);
 * - action reservations with deterministic ids and deferred mutations that
 *   apply only when their sibling action succeeds;
 * - per-thread standard <-> taskspace mode with scoped tool gating;
 * - context projection policies (`on-request` / `append` / `always`) built
 *   from DSH injection, dynamic prompt context and guidance sections;
 * - a client-visible projection view for a viewer or RPC carrier.
 *
 * Deleting this package restores DSH exactly; no upstream file is touched.
 * @module @deepseek-ai/dsh-whale-taskspace-spike
 */

import { Service } from '@deepseek-ai/cordis'
import type { Context } from '@deepseek-ai/cordis'
import { Remote, TypertRemoteService } from '@deepseek-ai/dsh-typert-protocol'
import type { Agent } from '@deepseek-ai/dsh-agent'
import { createUserMessage } from '@deepseek-ai/dsh-llm'
import { SessionId, appendPluginRecord, pluginRecordOf } from '@deepseek-ai/dsh-session'
import type { Session, SessionEvent } from '@deepseek-ai/dsh-session'
import { CommandDefinitionId } from '@deepseek-ai/dsh-commands'
import { defineTool } from '@deepseek-ai/dsh-tools'
import type { ToolExecution, ToolExecutionResult } from '@deepseek-ai/dsh-tools'
import { batchCallsOf, CONTROL_TOOL, validateBatch } from './batch.ts'
import type { BatchCall, BatchVerdict } from './batch.ts'
import { applyMutations, initialMap, renderMap } from './domain.ts'
import type { ExecuteManifest, InitPayload, MapSnapshot, Mutation } from './domain.ts'
import {
  ACTIONS_RECORD,
  BINDING_RECORD,
  DEFAULT_POLICY,
  MAP_RECORD,
  MODE_RECORD,
  POLICY_RECORD,
  taskspaceProjection,
} from './records.ts'
import type { ActionEntry, ActionsSnapshot, BindingRecord, PolicyRecord, TaskspaceMode, TaskspaceState, TaskspaceView } from './records.ts'

declare module '@deepseek-ai/cordis' {
  interface Context {
    /** The spike control surface (`ctx.whaleTaskspaceSpike`). */
    whaleTaskspaceSpike: WhaleTaskspaceSpike
  }
}

interface SessionRuntime {
  agent?: Agent
  restriction?: () => void
}

/** Prompt-section order for the TaskMap policy guidance (between plan and team). */
const POLICY_SECTION_ORDER = 650
/** Dynamic-context order for the always-projected map (after delegation context). */
const MAP_CONTEXT_ORDER = 130
/** Bounded action ledger entries retained per map. */
const ACTION_LEDGER_LIMIT = 50

const GUIDANCE = [
  'TaskMap is active in this session.',
  `Write map state ONLY through ${CONTROL_TOOL}:`,
  `- ${CONTROL_TOOL} must be the first call of its assistant message when action="execute",`,
  '  and every following sibling call must match manifest.actions in count, order and tool name;',
  '- action="read_map" | "initialize" | "finish_map" | "reopen_map" must be the only call;',
  '- manifest.expected_revision is the compare-and-set token for the revision you read.',
].join('\n')

/**
 * The spike: a service plugin owning the control tool, the sibling probe, the
 * batch guard, mode gating, reservations, context projection and the TaskMap
 * projection.
 */
export class WhaleTaskspaceSpike extends TypertRemoteService {
  static inject = ['tools', 'sessions', 'sessionProjections', 'systemPrompt']

  private readonly runtimes = new Map<SessionId, SessionRuntime>()
  private probeCount = 0

  constructor(ctx: Context) {
    super(ctx, 'whaleTaskspaceSpike')
    this.ctx.sessionProjections.register(taskspaceProjection)
    this.ctx.tools.guard(exec => this.denyReason(exec))
    this.ctx.on('agent/created', ({ agent }) => this.attachAgent(agent))
    this.ctx.on('session/event', (session, event) => this.onSessionEvent(session, event))
    this.ctx.on('tools/result', (exec, result) => this.onToolResult(exec, result))
    this.ctx.systemPrompt.section({
      name: 'whale-taskspace:policy',
      order: POLICY_SECTION_ORDER,
      text: context => this.guidance(context),
    })
    this.ctx.systemPrompt.context({
      name: 'whale-taskspace:map',
      order: MAP_CONTEXT_ORDER,
      text: context => this.alwaysContext(context),
    })
    this.ctx.tools.register(defineTool({
      name: CONTROL_TOOL,
      description: 'TaskMap control plane: read the canonical map, initialize one, commit an execute manifest, or finish/reopen it.',
      parameters: {
        action: { type: 'string', required: true, description: 'read_map | initialize | execute | finish_map | reopen_map' },
        manifest: { type: 'string', description: 'JSON payload required by initialize and execute' },
      },
      output: {
        schema: { type: 'string' },
        render: (_args, value) => [{ type: 'text', text: value }],
      },
      execute: async (args, exec) => this.control(args.action, args.manifest, exec),
    }))
    this.ctx.tools.register(defineTool({
      name: 'probe',
      description: 'Spike sibling tool: records one observable side effect and reports it.',
      parameters: {
        label: { type: 'string', required: true },
        fail: { type: 'boolean', description: 'When true the probe fails after recording the attempt.' },
        hang: { type: 'boolean', description: 'When true the probe stays in flight until cancelled.' },
      },
      isConcurrencySafe: () => true,
      output: {
        schema: { type: 'string' },
        render: (_args, value) => [{ type: 'text', text: value }],
      },
      execute: async (args, exec) => {
        this.probeCount += 1
        if (args.hang === true) {
          await new Promise((_resolve, reject) => {
            if (exec.signal.aborted) { reject(new Error(`probe ${args.label} aborted`)); return }
            exec.signal.addEventListener('abort', () => reject(new Error(`probe ${args.label} aborted`)), { once: true })
          })
        }
        if (args.fail === true) throw new Error(`probe ${args.label} failed`)
        return `probe:${args.label}`
      },
    }))
  }

  /** Number of `probe` sibling executions observed. */
  get probeRuns(): number {
    return this.probeCount
  }

  /**
   * Remote (source-mode) read of one session's TaskMap wire view — the seam an
   * external viewer or SDK client uses without a generated descriptor.
   * @param sessionId - exact session to read.
   * @returns the client view, or null when the session has no TaskMap.
   */
  @Remote('get')
  remoteGet(sessionId: string): TaskspaceView | null {
    const session = this.ctx.sessions.get(SessionId(sessionId))
    if (session === undefined) return null
    const snapshot = this.ctx.sessionProjections.snapshot(session, ['whaleTaskspace'])
    return (snapshot.values.whaleTaskspace as TaskspaceView | undefined) ?? null
  }

  /** Append one durable mode record; the plugin reacts and re-gates the agent. */
  setMode(session: Session, mode: TaskspaceMode): void {
    appendPluginRecord(session, MODE_RECORD, { mode })
  }

  /** Append one durable projection-policy record; omitted fields keep their current value. */
  setProjection(session: Session, policy: Partial<PolicyRecord>): PolicyRecord {
    const next: PolicyRecord = { ...this.policyOf(session), ...policy }
    appendPluginRecord(session, POLICY_RECORD, next)
    return next
  }

  /** Projected TaskMap state for one session, or undefined when unknown. */
  stateOf(session: Session): TaskspaceState | undefined {
    return this.ctx.sessionProjections.stateOf(session, 'whaleTaskspace')
  }

  /** Bounded map summary as the model context projection renders it. */
  summaryOf(session: Session): string | undefined {
    const { state } = this.resolved(session)
    if (state?.map === null || state?.map === undefined) return undefined
    return renderMap(state.map, state.policy.maxChars)
  }

  private policyOf(session: Session): PolicyRecord {
    return this.resolved(session).state?.policy ?? DEFAULT_POLICY
  }

  private modeOf(session: Session): TaskspaceMode {
    return this.stateOf(session)?.mode ?? 'standard'
  }

  private runtimeOf(sessionId: SessionId): SessionRuntime {
    const existing = this.runtimes.get(sessionId)
    if (existing !== undefined) return existing
    const created: SessionRuntime = {}
    this.runtimes.set(sessionId, created)
    return created
  }

  /**
   * Bind one member session to another session's canonical map: the member
   * writes through the owner's log (the owner stays the only authority) and
   * gains taskspace mode. The binding itself is a durable record on the member
   * log, so it survives restart and replay.
   * @param member - participating session (for example a delegated child).
   * @param owner - session whose map is the authority.
   */
  bindTeam(member: Session, owner: Session): BindingRecord {
    const map = this.stateOf(owner)?.map
    if (map === null || map === undefined) throw new Error('the team owner has no active map')
    const binding: BindingRecord = { version: 1, ownerSessionId: String(owner.header.id), mapId: map.mapId }
    appendPluginRecord(member, BINDING_RECORD, binding)
    appendPluginRecord(member, MODE_RECORD, { mode: 'taskspace' })
    return binding
  }

  /** Resolve the session that owns the map this session reads and writes. */
  private resolved(session: Session): { owner: Session; state: TaskspaceState | undefined } {
    const binding = this.stateOf(session)?.binding
    if (binding === null || binding === undefined) return { owner: session, state: this.stateOf(session) }
    const owner = this.ctx.sessions.get(SessionId(binding.ownerSessionId))
    if (owner === undefined) return { owner: session, state: this.stateOf(session) }
    return { owner, state: this.stateOf(owner) }
  }

  private agentOf(session: Session): Agent | undefined {
    return this.runtimes.get(session.header.id)?.agent
  }

  private attachAgent(agent: Agent): void {
    this.runtimeOf(agent.session.header.id).agent = agent
    this.applyMode(agent.session.header.id)
  }

  private onSessionEvent(session: Session, event: SessionEvent): void {
    if (pluginRecordOf(event)?.type !== MODE_RECORD) return
    this.applyMode(session.header.id)
  }

  /** Enforce mode visibility: standard hides the control tool for this agent only. */
  private applyMode(sessionId: SessionId): void {
    const runtime = this.runtimes.get(sessionId)
    if (runtime?.agent === undefined) return
    const mode = this.modeOf(runtime.agent.session)
    if (mode === 'taskspace') {
      runtime.restriction?.()
      runtime.restriction = undefined
      return
    }
    if (runtime.restriction !== undefined) return
    runtime.restriction = runtime.agent.ctx.tools.restrict({ deny: [CONTROL_TOOL] })
  }

  /** The batch verdict for the assistant message that carries `callId`. */
  private batchVerdict(agent: Agent | undefined, callId: string): BatchVerdict | undefined {
    if (agent === undefined) return undefined
    const calls = this.batchOf(agent, callId)
    return calls === undefined ? undefined : validateBatch(calls)
  }

  private batchOf(agent: Agent, callId: string): BatchCall[] | undefined {
    const events = agent.session.snapshotEvents()
    for (let index = events.length - 1; index >= 0; index -= 1) {
      const event = events[index]!
      if (event.type !== 'assistant/message') continue
      const calls = batchCallsOf(event.data.message.content)
      if (!calls.some(call => call.id === callId)) return undefined
      return calls
    }
    return undefined
  }

  /** Sibling calls of the batch whose control call is `callId`, in model order. */
  private siblingsOf(agent: Agent, callId: string): BatchCall[] {
    const calls = this.batchOf(agent, callId) ?? []
    const control = calls.findIndex(call => call.id === callId)
    return control < 0 ? [] : calls.slice(control + 1)
  }

  private denyReason(exec: ToolExecution): string | undefined {
    const agent = exec.agent
    if (agent !== undefined && exec.name === CONTROL_TOOL && this.modeOf(agent.session) !== 'taskspace') {
      return `${CONTROL_TOOL} is unavailable in standard mode`
    }
    const verdict = this.batchVerdict(agent, String(exec.callId))
    if (verdict?.kind === 'invalid') return `TaskMap batch rejected: ${verdict.reason}`
    if (verdict?.kind !== 'valid' || agent === undefined) return undefined
    return this.stateGate(agent.session, verdict)
  }

  /** State preconditions that must reject the whole batch before any dispatch. */
  private stateGate(session: Session, verdict: Extract<BatchVerdict, { kind: 'valid' }>): string | undefined {
    const map = this.resolved(session).state?.map ?? null
    if (verdict.action === 'initialize') {
      return map === null ? undefined : `map "${map.mapId}" is already active`
    }
    if (map === null) return 'no canonical map is active'
    if (verdict.action === 'execute') {
      const manifest = verdict.manifest!
      if (map.terminal) return `map "${map.mapId}" is terminal; reopen before mutating`
      if (manifest.mapId !== map.mapId) return `manifest mapId "${manifest.mapId}" does not match map "${map.mapId}"`
      if (manifest.expectedRevision !== undefined && manifest.expectedRevision !== map.revision) {
        return `revision conflict: expected ${manifest.expectedRevision} but map is at ${map.revision}`
      }
    }
    if (verdict.action === 'finish_map') {
      const unfinished = map.nodes.filter(node => node.status !== 'done')
      if (unfinished.length > 0) {
        return `map "${map.mapId}" has ${unfinished.length} unfinished node(s): ${unfinished.map(node => node.id).join(', ')}`
      }
      if (map.terminal) return `map "${map.mapId}" is already terminal`
    }
    if (verdict.action === 'reopen_map' && !map.terminal) return `map "${map.mapId}" is not terminal`
    return undefined
  }

  private guidance(context: { agent?: Agent }): string {
    const agent = context.agent
    if (agent === undefined) return ''
    return this.modeOf(agent.session) === 'taskspace' ? GUIDANCE : ''
  }

  private alwaysContext(context: { agent?: Agent }): string {
    const agent = context.agent
    if (agent === undefined) return ''
    const { state } = this.resolved(agent.session)
    if (state === undefined || state.map === null) return ''
    if (state.mode !== 'taskspace' || state.policy.projection !== 'always') return ''
    return renderMap(state.map, state.policy.maxChars)
  }

  /** Append the committed map summary after changes under the `append` policy. */
  private afterCommit(owner: Session): void {
    const agent = this.agentOf(owner)
    if (agent === undefined) return
    const state = this.stateOf(owner)
    if (state === undefined || state.map === null) return
    if (state.mode !== 'taskspace' || state.policy.projection !== 'append') return
    const last = state.actions.at(-1)
    const tail = last === undefined ? '' : `\nlast action ${last.id} ${last.state}`
    const text = `${renderMap(state.map, state.policy.maxChars)}${tail}`
    agent.inject(createUserMessage({
      content: [{ type: 'text', text }],
      source: { kind: 'whale-taskspace', form: 'instructions' },
    }))
  }

  private control(action: string, rawManifest: string | undefined, exec: ToolExecution): string {
    const agent = exec.agent
    const session = agent?.session
    if (agent === undefined || session === undefined) throw new Error(`${CONTROL_TOOL} requires an agent session`)
    const { owner, state } = this.resolved(session)
    const map = state?.map ?? null
    switch (action) {
      case 'read_map': {
        if (map === null) throw new Error('no canonical map is active')
        return JSON.stringify({ map, actions: state?.actions ?? [] })
      }
      case 'initialize': {
        if (map !== null) throw new Error(`map "${map.mapId}" is already active`)
        const snapshot = initialMap(this.parse<InitPayload>(rawManifest, 'initialize payload'))
        appendPluginRecord(owner, MAP_RECORD, snapshot)
        this.afterCommit(owner)
        return JSON.stringify({ kind: 'receipt', operation: 'initialize', mapId: snapshot.mapId, revision: snapshot.revision })
      }
      case 'execute': {
        if (map === null) throw new Error('no canonical map is active')
        const manifest = this.parse<ExecuteManifest>(rawManifest, 'execute manifest')
        const deferredByAction = new Map((manifest.deferred ?? []).map(deferral => [deferral.actionIndex, manifest.mutations[deferral.mutationIndex]!]))
        const deferredIndexes = new Set((manifest.deferred ?? []).map(deferral => deferral.mutationIndex))
        const immediate = manifest.mutations.filter((_mutation, index) => !deferredIndexes.has(index))
        let next = map
        if (immediate.length > 0) {
          next = applyMutations(map, immediate)
          appendPluginRecord(owner, MAP_RECORD, next)
        }
        const siblings = this.siblingsOf(agent, String(exec.callId))
        const entries: ActionEntry[] = manifest.actions.map((declaration, index) => {
          const sibling = siblings[index]
          const deferredMutation = deferredByAction.get(index)
          return {
            id: `action:${map.mapId}:${String(exec.callId)}:${index}`,
            callId: sibling === undefined ? `${String(exec.callId)}:missing:${index}` : sibling.id,
            tool: declaration.tool,
            ...declaration.nodeId === undefined ? {} : { nodeId: declaration.nodeId },
            state: 'reserved',
            ...deferredMutation === undefined ? {} : { deferredMutation },
          }
        })
        this.appendActions(owner, next.mapId, [...(state?.actions ?? []), ...entries])
        this.afterCommit(owner)
        return JSON.stringify({
          kind: 'receipt',
          operation: 'execute',
          mapId: next.mapId,
          revision: next.revision,
          reservations: entries.map(entry => entry.id),
        })
      }
      case 'finish_map': {
        if (map === null) throw new Error('no canonical map is active')
        const unfinished = map.nodes.filter(node => node.status !== 'done')
        if (unfinished.length > 0) {
          throw new Error(`map "${map.mapId}" has ${unfinished.length} unfinished node(s): ${unfinished.map(node => node.id).join(', ')}`)
        }
        const finished: MapSnapshot = { ...map, revision: map.revision + 1, terminal: true }
        appendPluginRecord(owner, MAP_RECORD, finished)
        this.afterCommit(owner)
        return JSON.stringify({ kind: 'receipt', operation: 'finish_map', mapId: finished.mapId, revision: finished.revision })
      }
      case 'reopen_map': {
        if (map === null) throw new Error('no canonical map is active')
        if (!map.terminal) throw new Error(`map "${map.mapId}" is not terminal`)
        const reopened: MapSnapshot = { ...map, revision: map.revision + 1, terminal: false }
        appendPluginRecord(owner, MAP_RECORD, reopened)
        this.afterCommit(owner)
        return JSON.stringify({ kind: 'receipt', operation: 'reopen_map', mapId: reopened.mapId, revision: reopened.revision })
      }
      default:
        throw new Error(`unknown ${CONTROL_TOOL} action ${JSON.stringify(action)}`)
    }
  }

  private appendActions(session: Session, mapId: string, entries: readonly ActionEntry[]): void {
    const snapshot: ActionsSnapshot = {
      version: 1,
      mapId,
      entries: entries.slice(Math.max(0, entries.length - ACTION_LEDGER_LIMIT)),
    }
    appendPluginRecord(session, ACTIONS_RECORD, snapshot)
  }

  /** Release one reservation when its sibling settles; apply a deferred mutation on success. */
  private onToolResult(exec: Readonly<ToolExecution>, result: Readonly<ToolExecutionResult>): void {
    const agent = exec.agent
    if (agent === undefined) return
    const { owner, state } = this.resolved(agent.session)
    if (state?.map === null || state?.map === undefined) return
    const index = state.actions.findIndex(entry => entry.callId === String(exec.callId) && entry.state === 'reserved')
    if (index < 0) return
    const entry = state.actions[index]!
    const ok = result.isError === false
    let map = state.map
    let next: ActionEntry = { ...entry, state: ok ? 'released' : 'failed' }
    if (ok && entry.deferredMutation !== undefined) {
      try {
        map = applyMutations(map, [entry.deferredMutation])
        next = { ...next, appliedMutation: entry.deferredMutation }
      } catch {
        next = { ...next, state: 'failed' }
      }
    }
    if (map !== state.map) appendPluginRecord(owner, MAP_RECORD, map)
    const entries = state.actions.map((candidate, position) => position === index ? next : candidate)
    this.appendActions(owner, map.mapId, entries)
    this.afterCommit(owner)
  }

  private parse<T>(raw: string | undefined, label: string): T {
    if (typeof raw !== 'string') throw new Error(`${label} is required`)
    try {
      return JSON.parse(raw) as T
    } catch {
      throw new Error(`${label} is not valid JSON`)
    }
  }
}

/**
 * Optional command surface (`/taskspace`, `/map-projection`) as its own
 * capability plugin: commands live in a separate role package, matching the
 * DSH one-role-per-package seam discipline and keeping the service free of a
 * hard commands dependency.
 */
export const taskspaceCommands = {
  name: 'whale-taskspace-commands',
  inject: ['commands', 'whaleTaskspaceSpike'],
  apply: (ctx: Context): void => {
    ctx.commands.register({
      definitionId: CommandDefinitionId('@whale/taskspace'),
      name: 'taskspace',
      description: 'Enter TaskMap mode, or leave it with /taskspace off.',
      input: { hint: '[off]' },
      handler: ({ agent, rawInput }) => {
        const off = rawInput.trim() === 'off'
        ctx.whaleTaskspaceSpike.setMode(agent.session, off ? 'standard' : 'taskspace')
        return { kind: 'success', text: off ? 'TaskMap mode off.' : 'TaskMap mode on.' }
      },
    })
    ctx.commands.register({
      definitionId: CommandDefinitionId('@whale/taskspace-projection'),
      name: 'map-projection',
      description: 'Set the TaskMap context projection (request|append|always).',
      input: { hint: 'request|append|always' },
      handler: ({ agent, rawInput }) => {
        const value = rawInput.trim()
        const projection = value === 'request' ? 'on-request' : value === 'append' || value === 'always' ? value : undefined
        if (projection === undefined) return { kind: 'error', text: 'Usage: /map-projection request|append|always' }
        ctx.whaleTaskspaceSpike.setProjection(agent.session, { projection })
        return { kind: 'success', text: `TaskMap projection: ${projection}` }
      },
    })
  },
}

/** Re-export the seams a consumer or test needs. */
export { CONTROL_TOOL, validateBatch } from './batch.ts'
export { applyMutations, countsOf, frontierOf, initialMap, renderMap } from './domain.ts'
export { ACTIONS_RECORD, BINDING_RECORD, MAP_RECORD, MODE_RECORD, POLICY_RECORD, taskspaceProjection } from './records.ts'
export type { ActionEntry, ActionsSnapshot, BindingRecord, PolicyRecord, TaskspaceMode, TaskspaceState, TaskspaceView } from './records.ts'
export type { ExecuteManifest, InitPayload, MapSnapshot, Mutation, WorkNode } from './domain.ts'
