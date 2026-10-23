import { describe, expect, test } from "vitest";
import { convexTest } from "convex-test";
import schema from "./schema";
import { api } from "./_generated/api";

const modules = import.meta.glob(["./**/*.ts", "!./**/*.test.ts"]);
function setup() {
  const backend = convexTest(schema, modules);
  return {
    backend,
    alice: backend.withIdentity({
      subject: "alice",
      issuer: "https://clerk.test",
      name: "Alice Example",
    }),
    bob: backend.withIdentity({
      subject: "bob",
      issuer: "https://clerk.test",
      name: "Bob Example",
    }),
  };
}
const submission = {
  fileName: "original-ideas.txt",
  fileSize: 120,
  course: "Academic Writing",
};
const preferences = {
  name: "Alice",
  role: "Educator",
  institution: "Example institution",
  language: "English",
  reminders: true,
  resources: false,
};

describe("authenticated Convex workspace", () => {
  test("unauthenticated callers cannot read or mutate workspace data", async () => {
    const { backend } = setup();
    await expect(backend.query(api.documents.list, {})).rejects.toThrow(
      "Sign in",
    );
    await expect(
      backend.mutation(api.documents.createDemo, submission),
    ).rejects.toThrow("Sign in");
    await expect(
      backend.mutation(api.feedback.save, {
        documentKey: "draft",
        body: "Hello",
      }),
    ).rejects.toThrow("Sign in");
    await expect(
      backend.mutation(api.preferences.save, preferences),
    ).rejects.toThrow("Sign in");
  });
  test("submissions persist with server-owned identity and stay isolated by user", async () => {
    const { alice, bob } = setup();
    const id = await alice.mutation(api.documents.createDemo, submission);
    const documents = await alice.query(api.documents.list, {});
    expect(documents).toHaveLength(1);
    expect(documents[0]).toMatchObject({
      id,
      name: "original-ideas",
      author: "Alice Example",
      status: "Demo report",
      similarity: 8,
    });
    expect(await bob.query(api.documents.list, {})).toEqual([]);
    await expect(
      bob.mutation(api.documents.markReviewed, { id }),
    ).rejects.toThrow("Document not found");
    await alice.mutation(api.documents.markReviewed, { id });
    expect((await alice.query(api.documents.list, {}))[0].status).toBe(
      "Reviewed",
    );
  });
  test("sample loading is explicit and idempotent", async () => {
    const { alice, bob } = setup();
    expect(await alice.query(api.documents.list, {})).toEqual([]);
    expect(await alice.mutation(api.documents.loadSamples, {})).toEqual({
      inserted: 6,
    });
    expect(await alice.mutation(api.documents.loadSamples, {})).toEqual({
      inserted: 0,
    });
    expect(await alice.query(api.documents.list, {})).toHaveLength(6);
    expect(await bob.query(api.documents.list, {})).toEqual([]);
  });
  test("feedback updates persist without allowing cross-user document references", async () => {
    const { alice, bob } = setup();
    const id = await alice.mutation(api.documents.createDemo, submission);
    await alice.mutation(api.feedback.save, {
      documentKey: id,
      body: "  Add one cited example.  ",
    });
    expect(await alice.query(api.feedback.get, { documentKey: id })).toBe(
      "Add one cited example.",
    );
    await alice.mutation(api.feedback.save, {
      documentKey: id,
      body: "A useful revision.",
    });
    expect(await alice.query(api.feedback.get, { documentKey: id })).toBe(
      "A useful revision.",
    );
    await expect(
      bob.query(api.feedback.get, { documentKey: id }),
    ).rejects.toThrow("Document not found");
    await expect(
      bob.mutation(api.feedback.save, { documentKey: id, body: "Overwrite" }),
    ).rejects.toThrow("Document not found");
    await alice.mutation(api.feedback.save, {
      documentKey: "draft",
      body: "Private draft feedback",
    });
    expect(await bob.query(api.feedback.get, { documentKey: "draft" })).toBe(
      "",
    );
  });
  test("preferences are saved per identity and updated without duplicate rows", async () => {
    const { backend, alice, bob } = setup();
    await alice.mutation(api.preferences.save, preferences);
    await alice.mutation(api.preferences.save, {
      ...preferences,
      language: "Español",
    });
    expect(await alice.query(api.preferences.get, {})).toEqual({
      ...preferences,
      language: "Español",
    });
    expect(await bob.query(api.preferences.get, {})).toBeNull();
    expect(
      await backend.run((context) =>
        context.db.query("acadguardPreferences").collect(),
      ),
    ).toHaveLength(1);
  });
  test("invalid file metadata and empty feedback are rejected on the backend", async () => {
    const { alice } = setup();
    for (const bad of [
      { ...submission, fileName: "program.exe" },
      { ...submission, fileSize: 20 * 1024 * 1024 + 1 },
      { ...submission, fileSize: -1 },
      { ...submission, fileSize: 0.5 },
    ]) {
      await expect(
        alice.mutation(api.documents.createDemo, bad),
      ).rejects.toThrow("Choose a PDF");
    }
    await expect(
      alice.mutation(api.feedback.save, { documentKey: "draft", body: " " }),
    ).rejects.toThrow("1–10,000");
    await expect(
      alice.mutation(api.preferences.save, { ...preferences, role: "Admin" }),
    ).rejects.toThrow("check your workspace preferences");
    expect(await alice.query(api.documents.list, {})).toEqual([]);
  });
});
