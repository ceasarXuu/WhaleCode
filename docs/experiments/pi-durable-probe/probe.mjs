/**
 * TaskMap-on-pi-durable probe: session-scoped canonical document, atomic
 * commits, parent/child durable tasks with a waiting join, task graph, live
 * document watch, and cross-version document migration after reopen.
 *
 * No model provider is called; the model registry is present only because the
 * Harness requires one.
 */
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { BACKGROUND_CONTEXT } from '@earendil-works/chord/context'
import { createModels } from '@earendil-works/pi-ai/models'
import { Harness, createRegistry, defineDoc, defineExtension, defineTask } from '@earendil-works/pi-durable'
import { openNodeJsonlStorage } from '@earendil-works/pi-durable/storage/jsonl/node'

const context = BACKGROUND_CONTEXT
const dir = mkdtempSync(join(tmpdir(), 'pi-durable-probe-'))
const out = (label, value) => console.log(`${label}: ${JSON.stringify(value)}`)

// ---- TaskMap document, version 1 -------------------------------------------------
const TaskMapV1 = defineDoc({
  kind: 'probe.taskmap',
  version: 1,
  scope: 'session',
  initial: () => ({ revision: 0, nodes: [] }),
})

// ---- durable tasks: parent waits for children (failFast) -------------------------
const Child = defineTask({
  name: 'probe.child',
  version: 1,
  initial: input => ({ phase: 'run', card: input.card }),
  phases: {
    run: async (task, runtime, context) => {
      await new Promise(resolve => setTimeout(resolve, 60))
      await runtime.commit(async () => ({
        status: 'terminal',
        outcome: { status: 'completed', result: { card: task.state.card } },
      }), context)
    },
  },
  abort: async (_task, runtime, context) => {
    await runtime.commit(async () => ({ status: 'terminal', outcome: { status: 'aborted' } }), context)
  },
})

const Parent = defineTask({
  name: 'probe.parent',
  version: 1,
  initial: input => ({ phase: 'spawn', cards: input.cards }),
  phases: {
    spawn: async (task, runtime, context) => {
      await runtime.commit(async (tx) => {
        const ids = []
        for (const card of task.state.checkpoint.cards) {
          ids.push(await tx.createTask(Child, { card }, { ownership: { kind: 'task', taskId: task.id } }))
        }
        return { status: 'waiting', checkpoint: { phase: 'decide', ids }, on: ids, policy: 'failFast' }
      }, context)
    },
    decide: async (task, runtime, context) => {
      const outcomes = await runtime.outcomes(task.state.checkpoint.ids, context)
      await runtime.commit(async () => ({
        status: 'terminal',
        outcome: { status: 'completed', result: { settled: outcomes.map(o => o.status) } },
      }), context)
    },
  },
  abort: async (_task, runtime, context) => {
    await runtime.commit(async () => ({ status: 'terminal', outcome: { status: 'aborted' } }), context)
  },
})

// ---- open, commit, share, watch, task graph --------------------------------------
const models = createModels()
const registry = createRegistry()
registry.install(defineExtension({ name: 'probe-tasks', tasks: [Child, Parent] }))
const storage = await openNodeJsonlStorage(dir, context)
let harness = await Harness.open(storage, { models, registry }, context)
const root = await harness.root(context, { agent: { model: { provider: 'faux', modelId: 'faux-1' } } })

// One atomic commit: transcript entry + canonical map document together.
await root.commit(async (tx) => {
  const map = await tx.doc(TaskMapV1)
  map.revision += 1
  map.nodes.push({ id: 'n1', status: 'pending' })
}, context)
out('v1.snapshot', await harness.snapshot(TaskMapV1, context))

// A second conversation sees the same session-scoped map (shared authority).
const other = await harness.createConversation({ ownership: { kind: 'ownerless' } }, context)
out('shared.otherConversationSees', await harness.snapshot(TaskMapV1, context))

// Live document watch: the UI data path for a map panel.
const watch = await harness.watchDoc(TaskMapV1, context)
const frames = []
watch.start(async (value) => { frames.push(value.revision) })
await root.commit(async (tx) => {
  const map = await tx.doc(TaskMapV1)
  map.revision += 1
  map.nodes[0].status = 'done'
}, context)
await new Promise(resolve => setTimeout(resolve, 20))
out('watch.frames', frames)
await watch.stop()

// Parent task owns two children and waits for them.
harness.resume()
const parentId = await root.commit(async (tx) => tx.createTask(Parent, { cards: [1, 2] }, { ownership: { kind: 'conversation' } }), context)
await new Promise(resolve => setTimeout(resolve, 30))
const graph = await harness.taskGraph(context)
const graphFrames = []
const stopGraph = graph.subscribe(value => {
  const nodes = Object.values(value.tasks).map(node => ({
    kind: node.kind,
    status: node.state.status,
    owner: node.owner ?? null,
    on: node.state.on?.length ?? 0,
  }))
  if (nodes.length > 0) graphFrames.push(nodes)
})
await new Promise(resolve => setTimeout(resolve, 120))
out('taskGraph.frames', graphFrames)
stopGraph()
const settled = await harness.waitForTask(parentId, context)
out('parent.outcome', settled.state.outcome)
out('map.afterTasks', await harness.snapshot(TaskMapV1, context))

await harness.close(context)
out('closed', true)

// ---- reopen with a version-2 definition and migrate ------------------------------
const TaskMapV2 = defineDoc({
  kind: 'probe.taskmap',
  version: 2,
  scope: 'session',
  initial: () => ({ revision: 0, nodes: [], status: 'open' }),
  migrate: value => ({ ...value, status: 'open' }),
})
harness = await Harness.open(await openNodeJsonlStorage(dir, context), { models, registry }, context)
out('v2.migratedSnapshot', await harness.snapshot(TaskMapV2, context))
await harness.close(context)
rmSync(dir, { recursive: true, force: true })
