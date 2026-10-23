import { v, ConvexError } from "convex/values";
import { mutation, query } from "./_generated/server";
import { requireUser, requireDocument } from "./lib/auth";

export const get = query({
  args: { documentKey: v.string() },
  handler: async (context, { documentKey }) => {
    const user = await requireUser(context);
    if (documentKey !== "draft")
      await requireDocument(context, documentKey, user.subject);
    const feedback = await context.db
      .query("acadguardFeedback")
      .withIndex("by_owner_and_document", (q) =>
        q.eq("ownerId", user.subject).eq("documentKey", documentKey),
      )
      .unique();
    return feedback?.body ?? "";
  },
});

export const save = mutation({
  args: { documentKey: v.string(), body: v.string() },
  handler: async (context, { documentKey, body }) => {
    const user = await requireUser(context);
    if (documentKey !== "draft")
      await requireDocument(context, documentKey, user.subject);
    const text = body.trim();
    if (!text || text.length > 10000)
      throw new ConvexError("Feedback must contain 1–10,000 characters.");
    const existing = await context.db
      .query("acadguardFeedback")
      .withIndex("by_owner_and_document", (q) =>
        q.eq("ownerId", user.subject).eq("documentKey", documentKey),
      )
      .unique();
    if (existing)
      await context.db.patch(existing._id, {
        body: text,
        updatedAt: Date.now(),
      });
    else
      await context.db.insert("acadguardFeedback", {
        ownerId: user.subject,
        documentKey,
        body: text,
        updatedAt: Date.now(),
      });
  },
});
