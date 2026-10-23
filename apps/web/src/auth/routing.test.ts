import assert from "node:assert/strict";
import { test } from "vitest";
import { legacyDestination, authDestination } from "./routing.ts";

test("old workspace links migrate to protected routes with deep links intact", () => {
  assert.equal(legacyDestination("#/app"), "/dashboard");
  assert.equal(legacyDestination("#/app/reports"), "/dashboard/reports");
  assert.equal(
    legacyDestination("#/app/resources?article=1"),
    "/dashboard/resources?article=1",
  );
  assert.equal(legacyDestination("#/login"), "/auth/login");
  assert.equal(legacyDestination("#/"), "/");
});

test("marketing anchors and unrelated hashes are not routed into the workspace", () => {
  for (const value of [
    "#solutions",
    "#resources",
    "#/application",
    "#https://example.com",
  ]) {
    assert.equal(legacyDestination(value), null);
  }
});

test("authentication can return to a local workspace deep link", () => {
  assert.equal(
    authDestination("?redirect_url=%2Fdashboard%2Freports"),
    "/dashboard/reports",
  );
  assert.equal(
    authDestination("?redirect_url=%2Fdashboard%2Fresources%3Farticle%3D1"),
    "/dashboard/resources?article=1",
  );
  assert.equal(authDestination(""), "/dashboard");
});

test("redirect destinations reject external, protocol-relative, and disguised paths", () => {
  for (const value of [
    "https://example.com",
    "//example.com",
    "/dashboard.evil",
    "/dashboard\\evil",
    "/auth/login",
    "/dashboard\n//example.com",
  ]) {
    assert.equal(
      authDestination(`?redirect_url=${encodeURIComponent(value)}`),
      "/dashboard",
    );
  }
});

test("Clerk absolute return URLs are reduced to local workspace paths", () => {
  for (const origin of ["http://127.0.0.1:5173", "https://external.example"]) {
    assert.equal(
      authDestination(
        `?redirect_url=${encodeURIComponent(origin + "/dashboard/reports?view=1")}`,
      ),
      "/dashboard/reports?view=1",
    );
  }
  assert.equal(
    authDestination("?redirect_url=%2Fdashboard%2F..%2Fauth%2Flogin"),
    "/dashboard",
  );
});
