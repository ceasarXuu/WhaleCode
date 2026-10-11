/**
 * Shared test harness for the TaskMap spike: real agent loop, tool pipeline,
 * session log, projection registry and (optionally) JSONL persistence; the
 * only mock is the scripted LLM adapter.
 */

import { Context } from '@deepseek-ai/cordis'
import { ToolCallId, createUserMessage } from '@deepseek-ai/dsh-llm'
import type { StreamChunk } from '@deepseek-ai/dsh-llm'
import type { SessionEvent } from '@deepseek-ai/dsh-session'
import type { Agent } from '@deepseek-ai/dsh-agent'
import AgentLoop from '@deepseek-ai/dsh-agent-loop'
import JsonlSessionPersistence from '@deepseek-ai/dsh-session-persistence-jsonl'
import { mountAgentLoopTestDependencies } from '@deepseek-ai/dsh-agent-loop-testkit'
import { MockAdapter } from '../../../core/agent-loop/tests/mock-adapter.ts'
import { WhaleTaskspaceSpike } from '../src/index.ts'

export interface Harness {
  readonly ctx: Context
  readonly controller: WhaleTaskspaceSpike
}

/** Mount the spike on the real agent stack; `root` enables JSONL persistence. */
export async function harness(adapter: MockAdapter, root?: string): Promise<Harness> {
  const ctx = new Context()
  await mountAgentLoopTestDependencies(ctx)
  if (root !== undefined) await ctx.plugin(JsonlSessionPersistence, { root })
  await ctx.plugin(WhaleTaskspaceSpike)
  await ctx.plugin(AgentLoop, { agents: [] })
  ctx.llm.registerAdapter(['mock'], adapter)
  return { ctx, controller: ctx.whaleTaskspaceSpike }
}

/** One assistant message carrying several tool calls, in model order. */
export function multiToolCallResponse(calls: readonly { id: string; name: string; args: object }[]): StreamChunk[] {
  const chunks: StreamChunk[] = []
  calls.forEach((call, index) => {
    const rawId = ToolCallId(call.id)
    const argumentsJson = JSON.stringify(call.args)
    chunks.push(
      { type: 'block-start', index, blockType: 'tool-call' },
      { type: 'tool-call-delta', index, id: rawId, name: call.name, argumentsDelta: argumentsJson.slice(0, 4) },
      { type: 'tool-call-delta', index, id: rawId, name: call.name, argumentsDelta: argumentsJson.slice(4) },
      { type: 'block-end', index, block: { type: 'tool-call', id: rawId, name: call.name, arguments: argumentsJson } },
    )
  })
  chunks.push(
    { type: 'usage', usage: { inputTokens: 10, outputTokens: 5 } },
    { type: 'finish', reason: { kind: 'tool-calls' } },
  )
  return chunks
}

export function waitForIdle(ctx: Context, agent: Agent): Promise<void> {
  return new Promise(resolve => {
    const dispose = ctx.on('agent/status', ({ agent: subject, status }) => {
      if (subject === agent && status === 'idle') {
        dispose()
        resolve()
      }
    })
  })
}

export function send(agent: Agent, text: string): void {
  agent.followup(createUserMessage({ content: [{ type: 'text', text }], source: { kind: 'user' } }))
}

export function events(agent: Agent): readonly SessionEvent[] {
  return agent.session.snapshotEvents()
}

export function toolResults(agent: Agent): readonly { text: string; isError: boolean }[] {
  return events(agent).flatMap(event => event.type === 'tool/result'
    ? [{
        text: event.data.message.content.map(block => block.type === 'text' ? block.text : '').join(''),
        isError: event.data.message.isError === true,
      }]
    : [])
}

/** All text content of one captured model request, joined for assertions. */
export function requestText(request: { messages?: readonly unknown[] } | undefined): string {
  if (request?.messages === undefined) return ''
  return request.messages.flatMap(message => {
    const content = (message as { content?: readonly { type: string; text?: string }[] }).content
    return content === undefined ? [] : content.flatMap(block => block.type === 'text' && block.text !== undefined ? [block.text] : [])
  }).join('\n')
}

export function requestsContaining(requests: readonly { messages?: readonly unknown[] }[], needle: string): number[] {
  return requests.flatMap((request, index) => requestText(request).includes(needle) ? [index] : [])
}

export { JsonlSessionPersistence, MockAdapter }
