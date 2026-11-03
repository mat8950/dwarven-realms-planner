// Main skill tree rules (UI_SkillTreeSystem / UI_SkillTreeNode_MASTER): the root is always
// owned; a node can be bought next to an owned one (1 point); it can be refunded when every
// other owned node stays connected to the root.
import type { SkillSet, SkillTree, TreeOption } from './types'

const rowIndex = new WeakMap<SkillTree, Map<string, number>>()

/** Node index of each row (dropdown options map to their node). */
export function nodeOfRow(tree: SkillTree): Map<string, number> {
  let m = rowIndex.get(tree)
  if (!m) {
    m = new Map()
    tree.nodes.forEach((n, i) => n.options.forEach((o) => m!.set(o.row, i)))
    rowIndex.set(tree, m)
  }
  return m
}

/** Owned nodes -> owned row ('' for the root). */
export function ownedNodes(tree: SkillTree, rows: string[]): Map<number, string> {
  const idx = nodeOfRow(tree)
  const out = new Map<number, string>()
  tree.nodes.forEach((n, i) => n.root && out.set(i, ''))
  for (const r of rows) {
    const i = idx.get(r)
    if (i !== undefined) out.set(i, r)
  }
  return out
}

export function availableNodes(tree: SkillTree, owned: Map<number, string>): Set<number> {
  const out = new Set<number>()
  for (const i of owned.keys()) for (const j of tree.nodes[i].links) if (!owned.has(j)) out.add(j)
  return out
}

/** Whether `i` can be refunded: the other owned nodes stay connected to the root. */
export function canRefund(tree: SkillTree, owned: Map<number, string>, i: number): boolean {
  if (tree.nodes[i].root || !owned.has(i)) return false
  const root = tree.nodes.findIndex((n) => n.root)
  const seen = new Set([root])
  const stack = [root]
  while (stack.length) {
    for (const j of tree.nodes[stack.pop()!].links) {
      if (j !== i && owned.has(j) && !seen.has(j)) {
        seen.add(j)
        stack.push(j)
      }
    }
  }
  return seen.size === owned.size - 1
}

/** Shortest chain of unowned nodes from the owned tree to `target` (target included). */
export function pathTo(tree: SkillTree, owned: Map<number, string>, target: number): number[] | null {
  if (owned.has(target)) return null
  const prev = new Map<number, number>()
  const queue: number[] = []
  for (const i of owned.keys()) {
    for (const j of tree.nodes[i].links) {
      if (!owned.has(j) && !prev.has(j)) {
        prev.set(j, -1)
        queue.push(j)
      }
    }
  }
  for (let q = 0; q < queue.length; q++) {
    const i = queue[q]
    if (i === target) {
      const path = [i]
      for (let p = prev.get(i)!; p !== -1; p = prev.get(p)!) path.push(p)
      return path.reverse()
    }
    for (const j of tree.nodes[i].links) {
      if (!owned.has(j) && !prev.has(j)) {
        prev.set(j, i)
        queue.push(j)
      }
    }
  }
  return null
}

/** Distinct (tag, value) bonuses of the tree options: one impact probe each, plus a removal
 *  probe (-value) for the owned ones (near a cap, adding and removing differ a lot). */
export function treeProbes(tree: SkillTree, owned: Set<string>): [string, number][] {
  const seen = new Map<string, [string, number]>()
  for (const n of tree.nodes) {
    for (const o of n.options) {
      for (const [t, v] of o.attributes) {
        if (!v) continue
        seen.set(probeKey(t, v), [t, v])
        if (owned.has(o.row)) seen.set(probeKey(t, -v), [t, -v])
      }
    }
  }
  return [...seen.values()]
}

/** Probes of the other skills: one level more, and one level less for the learned ones. */
export function skillProbes(sets: SkillSet[], level: (table: string, row: string) => number): [string, number][] {
  const seen = new Map<string, [string, number]>()
  for (const set of sets) {
    for (const s of set.skills) {
      for (const [t, v] of s.attributes) {
        if (!v) continue
        seen.set(probeKey(t, v), [t, v])
        if (level(set.table, s.row) > 0) seen.set(probeKey(t, -v), [t, -v])
      }
    }
  }
  return [...seen.values()]
}

export const probeKey = (tag: string, value: number) => `${tag}|${value}`

export const optionLabel = (o: TreeOption) => o.name || o.row
