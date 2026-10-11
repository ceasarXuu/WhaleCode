/**
 * Durable records, projection and client view for the TaskMap spike.
 *
 * Every record is an experimental plugin record (`plugin:whale-taskspace/*`):
 * ignorable, invisible to the model surface, carried by fork/resume, and
 * readable by a reader that does not load this plugin.
 * @module @deepseek-ai/dsh-whale-taskspace-spike/records
 */

import { pluginRecordOf } from '@deepseek-ai/dsh-session'
import type { ProjectionDefinition } from '@deepseek-ai/dsh-session-projection'
import type { ContextFormed } from '@deepseek-ai/dsh-llm'
import { z } from 'zod'
import { countsOf, mapSchema, type MapSnapshot, type Mutation, type NodeStatus } from './domain.ts'

/** Durable whole-map record name owned by this spike. */
export const MAP_RECORD = 'plugin:whale-taskspace/map' as const
/** Durable mode record name owned by this spike. */
export const MODE_RECORD = 'plugin:whale-taskspace/mode' as const
/** Durable context-projection policy record name owned by this spike. */
export const POLICY_RECORD = 'plugin:whale-taskspace/policy' as const
/** Durable action-reservation snapshot record name owned by this spike. */
export const ACTIONS_RECORD = 'plugin:whale-taskspace/actions' as const
/** Durable team-binding record: this session participates in an owner's map. */
export const BINDING_RECORD = 'plugin:whale-taskspace/binding' as const

export type TaskspaceMode = 'standard' | 'taskspace'

/** How map state reaches the model context. */
export type ProjectionPolicy = 'on-request' | 'append' | 'always'

export interface PolicyRecord {
  readonly projection: ProjectionPolicy
  readonly maxChars: number
}

export const DEFAULT_POLICY: PolicyRecord = { projection: 'append', maxChars: 1200 }

export type ActionState = 'reserved' | 'released' | 'failed'

/** One sibling action reservation with its outcome attribution. */
export interface ActionEntry {
  readonly id: string
  readonly callId: string
  readonly tool: string
  readonly nodeId?: string
  readonly state: ActionState
  /** Mutation applied only after this action succeeded. */
  readonly deferredMutation?: Mutation
  readonly appliedMutation?: Mutation
}

/** Whole action ledger for one map; every mutation rewrites the complete value. */
export interface ActionsSnapshot {
  readonly version: 1
  readonly mapId: string
  readonly entries: readonly ActionEntry[]
}

/** One member session bound to another session's canonical map. */
export interface BindingRecord {
  readonly version: 1
  readonly ownerSessionId: string
  readonly mapId: string
}

export const policySchema = z.object({
  projection: z.enum(['on-request', 'append', 'always']),
  maxChars: z.number().int().min(64).max(100_000),
})

export const actionEntrySchema = z.object({
  id: z.string(),
  callId: z.string(),
  tool: z.string(),
  nodeId: z.string().optional(),
  state: z.enum(['reserved', 'released', 'failed']),
  deferredMutation: z.unknown().optional(),
  appliedMutation: z.unknown().optional(),
})

export const actionsSchema = z.object({
  version: z.literal(1),
  mapId: z.string(),
  entries: z.array(actionEntrySchema),
})

export const bindingSchema = z.object({
  version: z.literal(1),
  ownerSessionId: z.string(),
  mapId: z.string(),
})

/** Host fold state for the spike projection. */
export interface TaskspaceState {
  readonly mode: TaskspaceMode
  readonly policy: PolicyRecord
  readonly map: MapSnapshot | null
  readonly actions: readonly ActionEntry[]
  /** Set on a member session that writes through another session's map. */
  readonly binding: BindingRecord | null
}

/** Client-visible whole value served by `sessionProjections.snapshot()`. */
export interface TaskspaceView {
  readonly mapId: string | null
  readonly revision: number
  readonly goal: string
  readonly terminal: boolean
  readonly counts: { readonly pending: number; readonly active: number; readonly blocked: number; readonly done: number; readonly ready: number }
  readonly nodes: readonly { readonly id: string; readonly title: string; readonly status: NodeStatus }[]
  readonly lastAction: { readonly id: string; readonly tool: string; readonly state: ActionState } | null
}

export const taskspaceViewSchema = z.object({
  mapId: z.string().nullable(),
  revision: z.number(),
  goal: z.string(),
  terminal: z.boolean(),
  counts: z.object({
    pending: z.number(),
    active: z.number(),
    blocked: z.number(),
    done: z.number(),
    ready: z.number(),
  }),
  nodes: z.array(z.object({ id: z.string(), title: z.string(), status: z.enum(['pending', 'active', 'blocked', 'done']) })),
  lastAction: z.object({ id: z.string(), tool: z.string(), state: z.enum(['reserved', 'released', 'failed']) }).nullable(),
})

export const taskspaceStateSchema = z.object({
  mode: z.enum(['standard', 'taskspace']),
  policy: policySchema,
  map: mapSchema.nullable(),
  actions: z.array(actionEntrySchema),
  binding: bindingSchema.nullable(),
})

declare module '@deepseek-ai/dsh-session/types' {
  interface PluginRecordMap {
    'plugin:whale-taskspace/map': MapSnapshot
    'plugin:whale-taskspace/mode': { readonly mode: TaskspaceMode }
    'plugin:whale-taskspace/policy': PolicyRecord
    'plugin:whale-taskspace/actions': ActionsSnapshot
    'plugin:whale-taskspace/binding': BindingRecord
  }
}

declare module '@deepseek-ai/dsh-session-projection/types' {
  interface SessionProjectionStateMap {
    whaleTaskspace: TaskspaceState
  }
  interface SessionProjectionMap {
    whaleTaskspace: TaskspaceView
  }
}

declare module '@deepseek-ai/dsh-llm' {
  interface MessageSourceMap {
    /** Durable TaskMap context appended to a session by this spike. */
    'whale-taskspace': { kind: 'whale-taskspace' } & ContextFormed
  }
}

function viewOf(state: TaskspaceState): TaskspaceView {
  const map = state.map
  const last = state.actions.at(-1)
  return {
    mapId: map?.mapId ?? null,
    revision: map?.revision ?? 0,
    goal: map?.goal ?? '',
    terminal: map?.terminal ?? false,
    counts: map === null
      ? { pending: 0, active: 0, blocked: 0, done: 0, ready: 0 }
      : countsOf(map),
    nodes: map === null ? [] : map.nodes.map(node => ({ id: node.id, title: node.title, status: node.status })),
    lastAction: last === undefined ? null : { id: last.id, tool: last.tool, state: last.state },
  }
}

/**
 * The TaskMap projection: folds this plugin's own records into whole host
 * state plus one client-visible view for a viewer or RPC carrier.
 */
export const taskspaceProjection: ProjectionDefinition<'whaleTaskspace'> = {
  key: 'whaleTaskspace',
  stateSchema: taskspaceStateSchema,
  init: () => ({ mode: 'standard', policy: DEFAULT_POLICY, map: null, actions: [], binding: null }),
  apply: (state, event) => {
    const record = pluginRecordOf(event)
    if (record === undefined) return state
    if (record.type === MODE_RECORD) {
      const mode = (record.data as { readonly mode?: unknown } | null)?.mode
      if (mode === 'taskspace' || mode === 'standard') return { ...state, mode }
      return state
    }
    if (record.type === BINDING_RECORD) {
      const parsed = bindingSchema.safeParse(record.data)
      return parsed.success ? { ...state, binding: parsed.data } : state
    }
    if (record.type === POLICY_RECORD) {
      const parsed = policySchema.safeParse(record.data)
      return parsed.success ? { ...state, policy: parsed.data } : state
    }
    if (record.type === MAP_RECORD) {
      const parsed = mapSchema.safeParse(record.data)
      return parsed.success ? { ...state, map: parsed.data } : state
    }
    if (record.type === ACTIONS_RECORD) {
      const parsed = actionsSchema.safeParse(record.data)
      return parsed.success
        ? { ...state, actions: parsed.data.entries as readonly ActionEntry[] }
        : state
    }
    return state
  },
  wire: { viewSchema: taskspaceViewSchema, view: viewOf },
  stateVersion: 1,
}
