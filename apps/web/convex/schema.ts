import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  acadguardDocuments: defineTable({
    ownerId: v.string(),
    name: v.string(),
    course: v.string(),
    author: v.string(),
    initials: v.string(),
    date: v.string(),
    similarity: v.number(),
    ai: v.number(),
    status: v.string(),
    words: v.number(),
    sample: v.boolean(),
    fileName: v.optional(v.string()),
    fileSize: v.optional(v.number()),
  }).index("by_owner", ["ownerId"]),
  acadguardFeedback: defineTable({
    ownerId: v.string(),
    documentKey: v.string(),
    body: v.string(),
    updatedAt: v.number(),
  }).index("by_owner_and_document", ["ownerId", "documentKey"]),
  acadguardPreferences: defineTable({
    ownerId: v.string(),
    name: v.string(),
    role: v.string(),
    institution: v.string(),
    language: v.string(),
    reminders: v.boolean(),
    resources: v.boolean(),
    updatedAt: v.number(),
  }).index("by_owner", ["ownerId"]),
});
