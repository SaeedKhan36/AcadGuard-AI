import { ConvexError } from "convex/values";
import type { QueryCtx, MutationCtx } from "../_generated/server";

export async function requireUser(context: Pick<QueryCtx, "auth">) {
  const identity = await context.auth.getUserIdentity();
  if (!identity) throw new ConvexError("Sign in to access your workspace.");
  return identity;
}

export async function requireDocument(
  context: QueryCtx | MutationCtx,
  key: string,
  ownerId: string,
) {
  const id = context.db.normalizeId("acadguardDocuments", key);
  const document = id ? await context.db.get(id) : null;
  if (!document || document.ownerId !== ownerId)
    throw new ConvexError("Document not found.");
  return document;
}
