import { v, ConvexError } from "convex/values";
import { mutation, query } from "./_generated/server";
import { requireUser, requireDocument } from "./lib/auth";
import { demoDocuments } from "../shared/workspace";

export const list = query({
  args: {},
  handler: async (context) => {
    const user = await requireUser(context);
    const documents = await context.db
      .query("acadguardDocuments")
      .withIndex("by_owner", (q) => q.eq("ownerId", user.subject))
      .order("desc")
      .take(200);
    return documents.map(
      ({
        _id,
        name,
        course,
        author,
        initials,
        date,
        similarity,
        ai,
        status,
        words,
      }) => ({
        id: _id,
        name,
        course,
        author,
        initials,
        date,
        similarity,
        ai,
        status,
        words,
      }),
    );
  },
});

export const createDemo = mutation({
  args: { fileName: v.string(), fileSize: v.number(), course: v.string() },
  handler: async (context, args) => {
    const user = await requireUser(context);
    const fileName = args.fileName.trim();
    if (
      !/\.(pdf|docx|txt)$/i.test(fileName) ||
      fileName.length > 255 ||
      args.fileSize < 0 ||
      args.fileSize > 20 * 1024 * 1024 ||
      !Number.isFinite(args.fileSize) ||
      !Number.isInteger(args.fileSize)
    )
      throw new ConvexError("Choose a PDF, DOCX, or TXT file up to 20 MB.");
    if (!args.course.trim() || args.course.length > 150)
      throw new ConvexError("Choose a valid class.");
    const author = user.name || user.givenName || "Your account";
    return await context.db.insert("acadguardDocuments", {
      ownerId: user.subject,
      name: fileName.replace(/\.[^.]+$/, ""),
      course: args.course.trim(),
      author,
      initials: author
        .split(/\s+/)
        .slice(0, 2)
        .map((part) => part[0])
        .join("")
        .toUpperCase(),
      date: new Date().toISOString().slice(0, 10),
      similarity: 8,
      ai: 0,
      status: "Demo report",
      words: 2450,
      sample: true,
      fileName,
      fileSize: args.fileSize,
    });
  },
});

export const loadSamples = mutation({
  args: {},
  handler: async (context) => {
    const user = await requireUser(context);
    const existing = await context.db
      .query("acadguardDocuments")
      .withIndex("by_owner", (q) => q.eq("ownerId", user.subject))
      .first();
    if (existing) return { inserted: 0 };
    for (const { id: _id, ...document } of [...demoDocuments].reverse()) {
      await context.db.insert("acadguardDocuments", {
        ...document,
        ownerId: user.subject,
        sample: true,
      });
    }
    return { inserted: demoDocuments.length };
  },
});

export const markReviewed = mutation({
  args: { id: v.id("acadguardDocuments") },
  handler: async (context, { id }) => {
    const user = await requireUser(context);
    await requireDocument(context, id, user.subject);
    await context.db.patch(id, { status: "Reviewed" });
  },
});
