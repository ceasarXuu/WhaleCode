/**
 * TaskSpace spike plugin — a pure out-of-tree-style DSH plugin probing the
 * three suspected blockers:
 *
 * 1. batch-atomic admission (control-first manifest, zero dispatch on
 *    mismatch) with no core hook;
 * 2. live standard <-> taskspace mode switching through per-agent
 *    `tools.restrict()` + a monotonic guard;
 * 3. durable canonical map state as ignorable `plugin:` records folded by a
 *    session projection, replayable without any core file edit.
 *
 * Deleting this package restores DSH exactly; no upstream file is touched.
 * @module @deepseek-ai/dsh-whale-taskspace-spike
 */

import { Service } from '@deepseek-ai/cordis'
import type { Context } from '@deepseek-ai/cordis'
import type { Agent } from '@deepseek-ai/dsh-agent'
import type { ToolCallBlock } from '@deepseek-ai/dsh-llm'
import { appendPluginRecord, pluginRecordOf } from '@deepseek-ai/dsh-session'
import type { Session, SessionEvent, SessionId } from '@deepseek-ai/dsh-session'
import { defineTool } from '@deepseek-ai/dsh-tools'
import type { ToolExecution } from '@deepseek-ai/dsh-tools'
import {
  applyMutations,
  CONTROL_TOOL,
  MAP_RECORD,
  MODE_RECORD,
  taskspaceProjection,
  validateBatch,
} from './types.ts'
import type { BatchCall, BatchVerdict, ExecuteManifest, MapSnapshot, TaskspaceMode, TaskspaceState } from './types.ts'

declare module '@deepseek-ai/cordis' {
  interface Context {
    /** The spike control surface (`ctx.whaleTaskspaceSpike`). */
    whaleTaskspaceSpike: WhaleTaskspaceSpike
  }
}

interface SessionRuntime {
  mode: TaskspaceMode
  agent?: Agent
  restriction?: () => void
}

/** Extract the ordered tool-call blocks of one assistant message payload. */
function toolCallsOf(content: readonly { readonly type: string }[]): BatchCall[] {
  return content.flatMap(block => block.type === 'tool-call'
    ? [{
        id: String((block as ToolCallBlock).id),
        name: (block as ToolCallBlock).name,
        arguments: (block as ToolCallBlock).arguments,
      }]
    : [])
}

/**
 * The spike: a service plugin that owns the control tool, the sibling probe,
 * the batch guard, per-session mode gating and the TaskSpace projection.
 */
export class WhaleTaskspaceSpike extends Service {
  static inject = ['tools', 'sessionProjections']

  private readonly runtimes = new Map<SessionId, SessionRuntime>()
  private probeCount = 0

  constructor(ctx: Context) {
    super(ctx, 'whaleTaskspaceSpike')
    this.ctx.sessionProjections.register(taskspaceProjection)
    this.ctx.tools.guard(exec => this.denyReason(exec))
    this.ctx.on('agent/created', ({ agent }) => this.attachAgent(agent))
    this.ctx.on('session/event', (session, event) => this.onSessionEvent(session, event))
    this.ctx.tools.register(defineTool({
      name: CONTROL_TOOL,
      description: 'TaskSpace control plane: read the canonical map, initialize one, or commit an execute manifest.',
      parameters: {
        action: { type: 'string', required: true, description: 'read_map | initialize | execute' },
        manifest: { type: 'string', description: 'JSON manifest or initialization payload' },
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
      parameters: { label: { type: 'string', required: true } },
      isConcurrencySafe: () => true,
      output: {
        schema: { type: 'string' },
        render: (_args, value) => [{ type: 'text', text: value }],
      },
      execute: async (args) => {
        this.probeCount += 1
        return `probe:${args.label}`
      },
    }))
  }

  /** Number of `probe` sibling executions observed. */
  get probeRuns(): number {
    return this.probeCount
  }

  /** Append one durable mode record; the plugin reacts and re-gates the agent. */
  setMode(session: Session, mode: TaskspaceMode): void {
    appendPluginRecord(session, MODE_RECORD, { mode })
  }

  /** Projected TaskSpace state for one session, or undefined when unknown. */
  stateOf(session: Session): TaskspaceState | undefined {
    return this.ctx.sessionProjections.stateOf(session, 'whaleTaskspace')
  }

  private runtimeOf(sessionId: SessionId): SessionRuntime {
    const existing = this.runtimes.get(sessionId)
    if (existing !== undefined) return existing
    const created: SessionRuntime = { mode: 'standard' }
    this.runtimes.set(sessionId, created)
    return created
  }

  private attachAgent(agent: Agent): void {
    const runtime = this.runtimeOf(agent.session.header.id)
    runtime.agent = agent
    const projected = this.stateOf(agent.session)
    if (projected !== undefined) runtime.mode = projected.mode
    this.applyMode(agent.session.header.id)
  }

  private onSessionEvent(session: Session, event: SessionEvent): void {
    const record = pluginRecordOf(event)
    if (record?.type !== MODE_RECORD) return
    const mode = (record.data as { readonly mode?: unknown } | null)?.mode
    if (mode !== 'standard' && mode !== 'taskspace') return
    const runtime = this.runtimeOf(session.header.id)
    runtime.mode = mode
    this.applyMode(session.header.id)
  }

  /** Enforce mode visibility: standard hides the control tool for this agent only. */
  private applyMode(sessionId: SessionId): void {
    const runtime = this.runtimes.get(sessionId)
    if (runtime?.agent === undefined) return
    if (runtime.mode === 'taskspace') {
      runtime.restriction?.()
      runtime.restriction = undefined
      return
    }
    if (runtime.restriction !== undefined) return
    runtime.restriction = runtime.agent.ctx.tools.restrict({ deny: [CONTROL_TOOL] })
  }

  /**
   * The batch verdict for the assistant message that carries `callId`.
   * Reading it from the live log means the verdict is fixed before any
   * pre-execute/guard runs, so a rejected batch denies every sibling
   * independently and no call can slip through under a rolling scheduler.
   */
  private batchVerdict(agent: Agent | undefined, callId: string): BatchVerdict | undefined {
    if (agent === undefined) return undefined
    const events = agent.session.snapshotEvents()
    for (let index = events.length - 1; index >= 0; index -= 1) {
      const event = events[index]!
      if (event.type !== 'assistant/message') continue
      const calls = toolCallsOf(event.data.message.content)
      if (!calls.some(call => call.id === callId)) return undefined
      return validateBatch(calls)
    }
    return undefined
  }

  private denyReason(exec: ToolExecution): string | undefined {
    const agent = exec.agent
    if (agent !== undefined && exec.name === CONTROL_TOOL) {
      const mode = this.runtimes.get(agent.session.header.id)?.mode ?? 'standard'
      if (mode !== 'taskspace') return `${CONTROL_TOOL} is unavailable in standard mode`
    }
    const verdict = this.batchVerdict(agent, String(exec.callId))
    if (verdict?.kind === 'invalid') return `TaskSpace batch rejected: ${verdict.reason}`
    return undefined
  }

  private control(action: string, manifest: string | undefined, exec: ToolExecution): string {
    const session = exec.agent?.session
    if (session === undefined) throw new Error(`${CONTROL_TOOL} requires an agent session`)
    const current = this.stateOf(session)?.map ?? null
    switch (action) {
      case 'read_map': {
        if (current === null) throw new Error('no canonical map is active for this session')
        return JSON.stringify(current)
      }
      case 'initialize': {
        if (current !== null) throw new Error(`map "${current.mapId}" is already active`)
        const payload = this.parse<{ mapId: string; nodes: readonly { id: string; title: string }[] }>(manifest, 'initialize payload')
        const snapshot: MapSnapshot = {
          version: 1,
          mapId: payload.mapId,
          revision: 1,
          nodes: payload.nodes.map(node => ({ id: node.id, title: node.title, status: 'pending' as const })),
          edges: [],
          terminal: false,
        }
        appendPluginRecord(session, MAP_RECORD, snapshot)
        return JSON.stringify({ kind: 'receipt', operation: 'initialize', mapId: snapshot.mapId, revision: snapshot.revision })
      }
      case 'execute': {
        if (current === null) throw new Error('no canonical map is active for this session')
        const parsed = this.parse<ExecuteManifest>(manifest, 'execute manifest')
        const next = applyMutations(current, parsed)
        appendPluginRecord(session, MAP_RECORD, next)
        return JSON.stringify({ kind: 'receipt', operation: 'execute', mapId: next.mapId, revision: next.revision })
      }
      default:
        throw new Error(`unknown ${CONTROL_TOOL} action ${JSON.stringify(action)}`)
    }
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
