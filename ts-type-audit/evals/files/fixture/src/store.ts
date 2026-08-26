import type { Draft } from "./orders.js";

const stored = new Map<string, Draft>();

export function store(draft: Draft): void {
  if (!(draft.resolved && draft.token !== undefined && draft.token.length > 0)) {
    throw new Error(`draft ${draft.orderId} is not resolved`);
  }
  stored.set(draft.orderId, draft);
}

export function lookup(orderId: string, userId: string): Draft | undefined {
  const draft = stored.get(orderId);
  return draft?.userId === userId ? draft : undefined;
}
