import { v, ConvexError } from "convex/values";
import { mutation, query } from "./_generated/server";
import { requireUser } from "./lib/auth";

export const get = query({
  args: {},
  handler: async (context) => {
    const user = await requireUser(context);
    const row = await context.db
      .query("acadguardPreferences")
      .withIndex("by_owner", (q) => q.eq("ownerId", user.subject))
      .unique();
    if (!row) return null;
    const { name, role, institution, language, reminders, resources } = row;
    return { name, role, institution, language, reminders, resources };
  },
});
export const save = mutation({
  args: {
    name: v.string(),
    role: v.string(),
    institution: v.string(),
    language: v.string(),
    reminders: v.boolean(),
    resources: v.boolean(),
  },
  handler: async (context, preferences) => {
    const user = await requireUser(context);
    if (
      !preferences.name.trim() ||
      preferences.name.length > 150 ||
      preferences.institution.length > 250 ||
      !["Educator", "Student", "Researcher"].includes(preferences.role) ||
      !["English", "हिन्दी", "Español"].includes(preferences.language)
    )
      throw new ConvexError("Please check your workspace preferences.");
    const existing = await context.db
      .query("acadguardPreferences")
      .withIndex("by_owner", (q) => q.eq("ownerId", user.subject))
      .unique();
    const values = {
      ...preferences,
      name: preferences.name.trim(),
      institution: preferences.institution.trim(),
      updatedAt: Date.now(),
    };
    if (existing) await context.db.patch(existing._id, values);
    else
      await context.db.insert("acadguardPreferences", {
        ...values,
        ownerId: user.subject,
      });
  },
});
