/**
 * TaskMap batch admission: pure validation of one assistant message's tool
 * batch against the control-first contract. The verdict is computed from the
 * committed message, so every sibling can be denied independently before any
 * tool body runs.
 * @module @deepseek-ai/dsh-whale-taskspace-spike/batch
 */

import type { ToolCallBlock } from '@deepseek-ai/dsh-llm'
import type { ExecuteManifest, InitPayload } from './domain.ts'

/** Model-facing control tool name. */
export const CONTROL_TOOL = 'taskspace_control'

/** Control actions the batch contract admits. */
export type ControlAction = 'read_map' | 'initialize' | 'execute' | 'finish_map' | 'reopen_map'

/** One assistant-message tool call as the batch validator sees it. */
export interface BatchCall {
  readonly id: string
  readonly name: string
  readonly arguments: string
}

export type BatchVerdict =
  | { readonly kind: 'unmanaged' }
  | { readonly kind: 'valid'; readonly action: ControlAction; readonly manifest?: ExecuteManifest; readonly payload?: InitPayload }
  | { readonly kind: 'invalid'; readonly reason: string }

/** Extract the ordered tool-call blocks of one assistant message payload. */
export function batchCallsOf(content: readonly { readonly type: string }[]): BatchCall[] {
  return content.flatMap(block => block.type === 'tool-call'
    ? [{
        id: String((block as ToolCallBlock).id),
        name: (block as ToolCallBlock).name,
        arguments: (block as ToolCallBlock).arguments,
      }]
    : [])
}

function parseJson(value: string): { ok: true; value: unknown } | { ok: false; reason: string } {
  try {
    return { ok: true, value: JSON.parse(value) as unknown }
  } catch {
    return { ok: false, reason: 'arguments are not valid JSON' }
  }
}

function checkManifest(raw: unknown): { ok: true; manifest: ExecuteManifest } | { ok: false; reason: string } {
  if (typeof raw !== 'string') return { ok: false, reason: `${CONTROL_TOOL}.execute requires a JSON manifest string` }
  const parsed = parseJson(raw)
  if (!parsed.ok) return { ok: false, reason: `manifest ${parsed.reason}` }
  const manifest = parsed.value as Partial<ExecuteManifest> | null
  if (manifest === null || typeof manifest !== 'object') return { ok: false, reason: 'manifest must be an object' }
  if (typeof manifest.mapId !== 'string') return { ok: false, reason: 'manifest.mapId must be a string' }
  if (!Array.isArray(manifest.actions)) return { ok: false, reason: 'manifest.actions must be an array' }
  if (!Array.isArray(manifest.mutations)) return { ok: false, reason: 'manifest.mutations must be an array' }
  if (manifest.expectedRevision !== undefined && !Number.isInteger(manifest.expectedRevision)) {
    return { ok: false, reason: 'manifest.expectedRevision must be an integer' }
  }
  if (manifest.deferred !== undefined && !Array.isArray(manifest.deferred)) {
    return { ok: false, reason: 'manifest.deferred must be an array' }
  }
  for (const deferral of manifest.deferred ?? []) {
    if (!Number.isInteger(deferral.mutationIndex) || !Number.isInteger(deferral.actionIndex)) {
      return { ok: false, reason: 'manifest.deferred entries require integer mutationIndex and actionIndex' }
    }
    if (deferral.mutationIndex < 0 || deferral.mutationIndex >= manifest.mutations.length) {
      return { ok: false, reason: `manifest.deferred mutationIndex ${deferral.mutationIndex} is out of range` }
    }
    if (deferral.actionIndex < 0 || deferral.actionIndex >= manifest.actions.length) {
      return { ok: false, reason: `manifest.deferred actionIndex ${deferral.actionIndex} is out of range` }
    }
  }
  return { ok: true, manifest: manifest as ExecuteManifest }
}

function checkPayload(raw: unknown): { ok: true; payload: InitPayload } | { ok: false; reason: string } {
  if (typeof raw !== 'string') return { ok: false, reason: `${CONTROL_TOOL}.initialize requires a JSON payload string` }
  const parsed = parseJson(raw)
  if (!parsed.ok) return { ok: false, reason: `initialize payload ${parsed.reason}` }
  const payload = parsed.value as Partial<InitPayload> | null
  if (payload === null || typeof payload !== 'object') return { ok: false, reason: 'initialize payload must be an object' }
  if (typeof payload.mapId !== 'string') return { ok: false, reason: 'initialize payload.mapId must be a string' }
  if (!Array.isArray(payload.nodes)) return { ok: false, reason: 'initialize payload.nodes must be an array' }
  if (payload.goal !== undefined && typeof payload.goal !== 'string') return { ok: false, reason: 'initialize payload.goal must be a string' }
  return { ok: true, payload: payload as InitPayload }
}

/**
 * Validate one assistant message's complete tool batch against the TaskMap
 * control contract.
 * @param calls - every tool call of one assistant message, in model order.
 * @returns the verdict the guard and commit path consume.
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
  const manifest = (args as { manifest?: unknown }).manifest

  if (action === 'read_map' || action === 'initialize' || action === 'finish_map' || action === 'reopen_map') {
    if (calls.length !== 1) {
      return { kind: 'invalid', reason: `${CONTROL_TOOL}.${action} must be the only call in its response` }
    }
    if (action === 'initialize') {
      const payload = checkPayload(manifest)
      return payload.ok ? { kind: 'valid', action, payload: payload.payload } : { kind: 'invalid', reason: payload.reason }
    }
    return { kind: 'valid', action }
  }
  if (action !== 'execute') return { kind: 'invalid', reason: `unknown ${CONTROL_TOOL} action ${JSON.stringify(action)}` }
  if (controlIndex !== 0) return { kind: 'invalid', reason: `${CONTROL_TOOL}.execute must be the first call in its response` }

  const ids = new Set(calls.map(call => call.id))
  if (ids.size !== calls.length) return { kind: 'invalid', reason: 'batch call ids are not unique' }

  const checked = checkManifest(manifest)
  if (!checked.ok) return { kind: 'invalid', reason: checked.reason }
  const siblings = calls.slice(1)
  if (siblings.length !== checked.manifest.actions.length) {
    return {
      kind: 'invalid',
      reason: `manifest declares ${checked.manifest.actions.length} action(s) but the batch carries ${siblings.length} sibling call(s)`,
    }
  }
  for (let index = 0; index < siblings.length; index += 1) {
    const declared = checked.manifest.actions[index]?.tool
    const actual = siblings[index]!.name
    if (declared !== actual) {
      return { kind: 'invalid', reason: `manifest action ${index} expects tool "${String(declared)}" but sibling ${index} is "${actual}"` }
    }
  }
  return { kind: 'valid', action: 'execute', manifest: checked.manifest }
}
