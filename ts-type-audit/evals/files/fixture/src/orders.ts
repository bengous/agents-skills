export interface Draft {
  userId: string;
  orderId: string;
  token?: string;
  resolved: boolean;
}

export function attachOrder(userId: string, orderId: string): Draft {
  return { userId, orderId, resolved: false };
}

export function resolveDraft(draft: Draft, token: string): Draft {
  return { ...draft, token, resolved: true };
}

export function canStore(draft: Draft): boolean {
  return draft.resolved && draft.token !== undefined && draft.token.length > 0;
}
