/**
 * Spike types for a TaskSpace plugin built ONLY on public DSH seams.
 *
 * Every durable record here is an experimental plugin record
 * (`plugin:whale-taskspace/*`), never a `SessionEventMap` member: an
 * out-of-repo plugin cannot edit core session types, so the spike proves the
 * `PluginRecordMap` + `ignorable` path is sufficient for canonical map state.
 * @module @deepseek-ai/dsh-whale-taskspace-spike/types
 */

import { z } from 'zod'
import { pluginRecordOf } from '@deepseek-ai/dsh-session'
import type { SessionEvent } from '@deepseek-ai/dsh-session'
import type { ProjectionDefinition } from '@deepseek-ai/dsh-session-projection'

/** Durable whole-map record name owned by this spike. */
export const MAP_RECORD = 'plugin:whale-taskspace/map' as const
/** Durable mode record name owned by this spike. */
export const MODE_RECORD = 'plugin:whale-taskspace/mode' as const

/** Model-facing control tool name. */
export const CONTROL_TOOL = 'taskspace_control'

export type TaskspaceMode = 'standard' | 'taskspace'
export type NodeStatus = 'pending' | 'active' | 'done' | 'blocked'

export interface WorkNode {
  readonly id: string
  readonly title: string
  readonly status: NodeStatus
}

export interface WorkEdge {
  readonly from: string
  readonly to: string
}

/** Canonical whole-map value: every commit stores the complete post-change snapshot. */
export interface MapSnapshot {
  readonly version: 1
  readonly mapId: string
  readonly revision: number
  readonly nodes: readonly WorkNode[]
  readonly edges: readonly WorkEdge[]
  readonly terminal: boolean
}

export const nodeSchema = z.object({
  id: z.string(),
  title: z.string(),
  status: z.enum(['pending', 'active', 'done', 'blocked']),
})

export const mapSchema = z.object({
  version: z.literal(1),
  mapId: z.string(),
  revision: z.number().int().min(1),
  nodes: z.array(nodeSchema),
  edges: z.array(z.object({ from: z.string(), to: z.string() })),
  terminal: z.boolean(),
})

/** Host fold state for the spike projection. */
export interface TaskspaceState {
  readonly mode: TaskspaceMode
  readonly map: MapSnapshot | null
}

export const taskspaceStateSchema = z.object({
  mode: z.enum(['standard', 'taskspace']),
  map: mapSchema.nullable(),
})

declare module '@deepseek-ai/dsh-session/types' {
  interface PluginRecordMap {
    'plugin:whale-taskspace/map': MapSnapshot
    'plugin:whale-taskspace/mode': { readonly mode: TaskspaceMode }
  }
}

declare module '@deepseek-ai/dsh-session-projection/types' {
  interface SessionProjectionStateMap {
    whaleTaskspace: TaskspaceState
  }
}

/** One assistant-message tool call as the batch validator sees it. */
export interface BatchCall {
  readonly id: string
  readonly name: string
  readonly arguments: string
}

/** One mutation the control tool applies on a successful execute commit. */
export type Mutation =
  | { readonly kind: 'add_work_nodes'; readonly nodes: readonly { readonly id: string; readonly title: string }[] }
  | { readonly kind: 'complete_node'; readonly nodeId: string }
  | { readonly kind: 'block_node'; readonly nodeId: string }
  | { readonly kind: 'unblock_node'; readonly nodeId: string }

/** The manifest a control `execute` call declares and the batch must match. */
export interface ExecuteManifest {
  readonly mapId: string
  readonly actions: readonly { readonly tool: string }[]
  readonly mutations: readonly Mutation[]
}

export type BatchVerdict =
  | { readonly kind: 'unmanaged' }
  | { readonly kind: 'valid'; readonly action: 'read_map' | 'initialize' | 'execute' }
  | { readonly kind: 'invalid'; readonly reason: string }

function parseJson(value: string): { ok: true; value: unknown } | { ok: false; reason: string } {
  try {
    return { ok: true, value: JSON.parse(value) as unknown }
  } catch {
    return { ok: false, reason: 'arguments are not valid JSON' }
  }
}

/**
 * Validate one assistant message's complete tool batch against the TaskSpace
 * control contract, the way the Rust kernel's batch preflight does:
 * `read_map`/`initialize` must be alone, `execute` must be first, and the
 * consecutive siblings must match `manifest.actions` in count, order and name.
 * @param calls - every tool call of one assistant message, in model order.
 * @returns the batch verdict the guard and commit path consume.
 */
export function validateBatch(calls: readonly BatchCall[]): BatchVerdict {
  const controlIndexes = calls.flatMap((call, index) => call.name === CONTROL_TOOL ? [index] : [])
  if (controlIndexes.length === 0) return { kind: 'unmanaged' }
  if (controlIndexes.length > 1) {
    return { kind: 'invalid', reason: `batch carries ${controlIndexes.length} ${CONTROL_TOOL} calls; exactly one is required` }
  }
  const controlIndex = controlIndexes[0]!
  const control = calls[controlIndex]!
  const parsed = parseJson(control.arguments)
  if (!parsed.ok) return { kind: 'invalid', reason: `${CONTROL_TOOL} ${parsed.reason}` }
  const args = parsed.value
  if (typeof args !== 'object' || args === null) return { kind: 'invalid', reason: `${CONTROL_TOOL} arguments must be an object` }
  const action = (args as { action?: unknown }).action

  if (action === 'read_map' || action === 'initialize') {
    if (calls.length !== 1) {
      return { kind: 'invalid', reason: `${CONTROL_TOOL}.${action} must be the only call in its response` }
    }
    return { kind: 'valid', action }
  }
  if (action !== 'execute') return { kind: 'invalid', reason: `unknown ${CONTROL_TOOL} action ${JSON.stringify(action)}` }
  if (controlIndex !== 0) return { kind: 'invalid', reason: `${CONTROL_TOOL}.execute must be the first call in its response` }

  const ids = new Set(calls.map(call => call.id))
  if (ids.size !== calls.length) return { kind: 'invalid', reason: 'batch call ids are not unique' }

  const rawManifest = (args as { manifest?: unknown }).manifest
  if (typeof rawManifest !== 'string') return { kind: 'invalid', reason: `${CONTROL_TOOL}.execute requires a JSON manifest string` }
  const manifestParsed = parseJson(rawManifest)
  if (!manifestParsed.ok) return { kind: 'invalid', reason: `manifest ${manifestParsed.reason}` }
  const manifest = manifestParsed.value as Partial<ExecuteManifest> | null
  if (manifest === null || typeof manifest !== 'object') return { kind: 'invalid', reason: 'manifest must be an object' }
  if (typeof manifest.mapId !== 'string') return { kind: 'invalid', reason: 'manifest.mapId must be a string' }
  if (!Array.isArray(manifest.actions)) return { kind: 'invalid', reason: 'manifest.actions must be an array' }
  if (!Array.isArray(manifest.mutations)) return { kind: 'invalid', reason: 'manifest.mutations must be an array' }

  const siblings = calls.slice(1)
  if (siblings.length !== manifest.actions.length) {
    return {
      kind: 'invalid',
      reason: `manifest declares ${manifest.actions.length} action(s) but the batch carries ${siblings.length} sibling call(s)`,
    }
  }
  for (let index = 0; index < siblings.length; index += 1) {
    const declared = manifest.actions[index]?.tool
    const actual = siblings[index]!.name
    if (declared !== actual) {
      return { kind: 'invalid', reason: `manifest action ${index} expects tool "${String(declared)}" but sibling ${index} is "${actual}"` }
    }
  }
  return { kind: 'valid', action: 'execute' }
}

/** Apply one validated manifest to a map; throws on a domain-invalid mutation. */
export function applyMutations(map: MapSnapshot, manifest: ExecuteManifest): MapSnapshot {
  if (map.mapId !== manifest.mapId) throw new Error(`manifest mapId "${manifest.mapId}" does not match map "${map.mapId}"`)
  if (map.terminal) throw new Error('map is terminal; reopen before mutating')
  const nodes = [...map.nodes]
  const find = (nodeId: string): number => {
    const index = nodes.findIndex(node => node.id === nodeId)
    if (index < 0) throw new Error(`node "${nodeId}" is not in map "${map.mapId}"`)
    return index
  }
  for (const mutation of manifest.mutations) {
    switch (mutation.kind) {
      case 'add_work_nodes': {
        for (const node of mutation.nodes) {
          if (nodes.some(existing => existing.id === node.id)) throw new Error(`node "${node.id}" already exists`)
          nodes.push({ id: node.id, title: node.title, status: 'pending' })
        }
        break
      }
      case 'complete_node': {
        const index = find(mutation.nodeId)
        nodes[index] = { ...nodes[index]!, status: 'done' }
        break
      }
      case 'block_node': {
        const index = find(mutation.nodeId)
        nodes[index] = { ...nodes[index]!, status: 'blocked' }
        break
      }
      case 'unblock_node': {
        const index = find(mutation.nodeId)
        nodes[index] = { ...nodes[index]!, status: 'active' }
        break
      }
      default: throw new Error(`unsupported mutation ${JSON.stringify(mutation)}`)
    }
  }
  return { ...map, revision: map.revision + 1, nodes }
}

/**
 * The spike projection: folds this plugin's own ignorable records into whole
 * host state, exactly like an in-tree domain contributor but declared
 * out-of-repo.
 */
export const taskspaceProjection: ProjectionDefinition<'whaleTaskspace'> = {
  key: 'whaleTaskspace',
  stateSchema: taskspaceStateSchema,
  init: () => ({ mode: 'standard', map: null }),
  apply: (state, event: SessionEvent) => {
    const record = pluginRecordOf(event)
    if (record === undefined) return state
    if (record.type === MODE_RECORD) {
      const mode = (record.data as { readonly mode?: unknown } | null)?.mode
      if (mode === 'taskspace' || mode === 'standard') return { ...state, mode }
      return state
    }
    if (record.type === MAP_RECORD) {
      const parsed = mapSchema.safeParse(record.data)
      return parsed.success ? { ...state, map: parsed.data } : state
    }
    return state
  },
  stateVersion: 1,
}
