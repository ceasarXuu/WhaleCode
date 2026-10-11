/**
 * TaskMap domain: canonical work-node DAG, mutation application and bounded
 * rendering. Domain rules are pure functions so the same code drives the
 * control tool, the projection fold and the replay path.
 * @module @deepseek-ai/dsh-whale-taskspace-spike/domain
 */

import { z } from 'zod'

export type NodeStatus = 'pending' | 'active' | 'blocked' | 'done'

export interface WorkNode {
  readonly id: string
  readonly title: string
  readonly status: NodeStatus
  /** Node ids that must be `done` before this node is ready. */
  readonly blockedBy: readonly string[]
}

/** Canonical whole-map value: every commit stores the complete post-change snapshot. */
export interface MapSnapshot {
  readonly version: 1
  readonly mapId: string
  readonly goal: string
  /** Compare-and-set token; every commit increments by one. */
  readonly revision: number
  readonly nodes: readonly WorkNode[]
  readonly terminal: boolean
}

/** One domain mutation the control plane may commit or defer to an action. */
export type Mutation =
  | { readonly kind: 'add_work_nodes'; readonly nodes: readonly { readonly id: string; readonly title: string; readonly blockedBy?: readonly string[] }[] }
  | { readonly kind: 'set_blocked_by'; readonly nodeId: string; readonly blockedBy: readonly string[] }
  | { readonly kind: 'activate_node'; readonly nodeId: string }
  | { readonly kind: 'complete_node'; readonly nodeId: string }
  | { readonly kind: 'block_node'; readonly nodeId: string }
  | { readonly kind: 'unblock_node'; readonly nodeId: string }

/** One model-declared sibling action and the node it works on. */
export interface ActionDeclaration {
  readonly tool: string
  readonly nodeId?: string
}

/** Deferral link: mutation `mutationIndex` applies only when action `actionIndex` succeeds. */
export interface Deferral {
  readonly mutationIndex: number
  readonly actionIndex: number
}

/** The manifest a control `execute` call declares and the batch must match. */
export interface ExecuteManifest {
  readonly mapId: string
  /** CAS token; when present it must equal the live map revision. */
  readonly expectedRevision?: number
  readonly actions: readonly ActionDeclaration[]
  readonly mutations: readonly Mutation[]
  readonly deferred?: readonly Deferral[]
}

/** Payload of one `initialize` control call. */
export interface InitPayload {
  readonly mapId: string
  readonly goal?: string
  readonly nodes: readonly { readonly id: string; readonly title: string; readonly blockedBy?: readonly string[] }[]
}

export const nodeSchema = z.object({
  id: z.string(),
  title: z.string(),
  status: z.enum(['pending', 'active', 'blocked', 'done']),
  blockedBy: z.array(z.string()),
})

export const mapSchema = z.object({
  version: z.literal(1),
  mapId: z.string(),
  goal: z.string(),
  revision: z.number().int().min(1),
  nodes: z.array(nodeSchema),
  terminal: z.boolean(),
})

/** Find one node index or throw a domain error that becomes a tool error. */
function indexOf(map: MapSnapshot, nodeId: string): number {
  const index = map.nodes.findIndex(node => node.id === nodeId)
  if (index < 0) throw new Error(`node "${nodeId}" is not in map "${map.mapId}"`)
  return index
}

/** Reject dependency cycles; node ids must already be unique. */
function assertAcyclic(nodes: readonly WorkNode[], mapId: string): void {
  const dependencies = new Map(nodes.map(node => [node.id, node.blockedBy]))
  const visiting = new Set<string>()
  const visited = new Set<string>()
  const walk = (id: string, trail: readonly string[]): void => {
    if (visited.has(id)) return
    if (visiting.has(id)) throw new Error(`map "${mapId}" dependency cycle: ${[...trail, id].join(' -> ')}`)
    visiting.add(id)
    for (const blocker of dependencies.get(id) ?? []) {
      if (!dependencies.has(blocker)) throw new Error(`node "${id}" is blocked by unknown node "${blocker}"`)
      walk(blocker, [...trail, id])
    }
    visiting.delete(id)
    visited.add(id)
  }
  for (const node of nodes) walk(node.id, [])
}

/**
 * Apply structural mutations to one map.
 * @param map - current canonical map.
 * @param mutations - ordered mutations in one commit.
 * @returns the next whole-map snapshot at `revision + 1`.
 */
export function applyMutations(map: MapSnapshot, mutations: readonly Mutation[]): MapSnapshot {
  if (map.terminal) throw new Error(`map "${map.mapId}" is terminal; reopen before mutating`)
  let nodes: WorkNode[] = map.nodes.map(node => ({ ...node, blockedBy: [...node.blockedBy] }))
  for (const mutation of mutations) {
    switch (mutation.kind) {
      case 'add_work_nodes': {
        for (const added of mutation.nodes) {
          if (nodes.some(node => node.id === added.id)) throw new Error(`node "${added.id}" already exists`)
          nodes.push({
            id: added.id,
            title: added.title,
            status: 'pending',
            blockedBy: added.blockedBy === undefined ? [] : [...added.blockedBy],
          })
        }
        break
      }
      case 'set_blocked_by': {
        const index = indexOf({ ...map, nodes }, mutation.nodeId)
        nodes[index] = { ...nodes[index]!, blockedBy: [...mutation.blockedBy] }
        break
      }
      case 'activate_node': {
        const index = indexOf({ ...map, nodes }, mutation.nodeId)
        const node = nodes[index]!
        const ready = node.blockedBy.every(blocker => nodes.find(candidate => candidate.id === blocker)?.status === 'done')
        if (!ready) throw new Error(`node "${node.id}" is not ready: blockers ${node.blockedBy.join(', ')} are unfinished`)
        nodes[index] = { ...node, status: 'active' }
        break
      }
      case 'complete_node': {
        const index = indexOf({ ...map, nodes }, mutation.nodeId)
        const node = nodes[index]!
        if (node.status === 'done') throw new Error(`node "${node.id}" is already done`)
        nodes[index] = { ...node, status: 'done' }
        break
      }
      case 'block_node': {
        const index = indexOf({ ...map, nodes }, mutation.nodeId)
        nodes[index] = { ...nodes[index]!, status: 'blocked' }
        break
      }
      case 'unblock_node': {
        const index = indexOf({ ...map, nodes }, mutation.nodeId)
        nodes[index] = { ...nodes[index]!, status: 'pending' }
        break
      }
      default: throw new Error(`unsupported mutation ${JSON.stringify(mutation)}`)
    }
  }
  assertAcyclic(nodes, map.mapId)
  return { ...map, revision: map.revision + 1, nodes }
}

/** Build the initial map for an `initialize` payload. */
export function initialMap(payload: InitPayload): MapSnapshot {
  if (payload.nodes.length === 0) throw new Error('initialize requires at least one work node')
  const nodes: WorkNode[] = payload.nodes.map(node => ({
    id: node.id,
    title: node.title,
    status: 'pending',
    blockedBy: node.blockedBy === undefined ? [] : [...node.blockedBy],
  }))
  const ids = new Set<string>()
  for (const node of nodes) {
    if (ids.has(node.id)) throw new Error(`node "${node.id}" appears twice`)
    ids.add(node.id)
  }
  assertAcyclic(nodes, payload.mapId)
  return { version: 1, mapId: payload.mapId, goal: payload.goal ?? '', revision: 1, nodes, terminal: false }
}

/** Node ids that are pending with every blocker done — the ready frontier. */
export function frontierOf(map: MapSnapshot): string[] {
  return map.nodes
    .filter(node => node.status === 'pending')
    .filter(node => node.blockedBy.every(blocker => map.nodes.find(candidate => candidate.id === blocker)?.status === 'done'))
    .map(node => node.id)
}

export interface MapCounts {
  readonly pending: number
  readonly active: number
  readonly blocked: number
  readonly done: number
  readonly ready: number
}

export function countsOf(map: MapSnapshot): MapCounts {
  const counts = { pending: 0, active: 0, blocked: 0, done: 0, ready: frontierOf(map).length }
  for (const node of map.nodes) counts[node.status] += 1
  return counts
}

/**
 * Render one bounded model-facing map summary.
 * @param map - canonical map.
 * @param maxChars - hard character cap; excess nodes become an ellipsis notice.
 * @returns a multi-line summary no longer than `maxChars` characters.
 */
export function renderMap(map: MapSnapshot, maxChars: number): string {
  const counts = countsOf(map)
  const header = `TaskMap "${map.mapId}" revision ${map.revision}${map.terminal ? ' [terminal]' : ''}`
    + `${map.goal === '' ? '' : ` goal="${map.goal}"`}`
  const progress = `progress: ${counts.done}/${map.nodes.length} done, ${counts.active} active, ${counts.ready} ready, ${counts.blocked} blocked`
  const lines = [header, progress]
  const ready = frontierOf(map)
  let omitted = 0
  for (const node of map.nodes) {
    const marker = node.status === 'done'
      ? 'done'
      : node.status === 'blocked'
        ? 'blocked'
        : node.status === 'active'
          ? 'active'
          : ready.includes(node.id) ? 'ready' : 'waiting'
    const dependencies = node.blockedBy.length === 0 ? '' : ` (blocked by ${node.blockedBy.join(', ')})`
    const line = `- ${node.id} [${marker}] ${node.title}${dependencies}`
    const optimistic = lines.reduce((total, existing) => total + existing.length + 1, line.length)
    if (optimistic > maxChars) {
      omitted += 1
      continue
    }
    lines.push(line)
  }
  if (omitted > 0) lines.push(`… (+${omitted} more node${omitted === 1 ? '' : 's'})`)
  const rendered = lines.join('\n')
  return rendered.length <= maxChars ? rendered : `${rendered.slice(0, Math.max(0, maxChars - 1))}…`
}
