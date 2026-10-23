import { describe, expect, it } from "vitest";
import { parseSearch, stringifySearch } from "./search";

describe("route query compatibility", () => {
  it("keeps numeric article IDs usable by the resource dialog", () => {
    expect(stringifySearch({ article: "0" })).toBe("?article=0");
    expect(parseSearch("?article=0").article).toBe("0");
  });
  it("preserves nested workspace return URLs across navigation", () => {
    const search = { redirect_url: "/dashboard/resources?article=2" };
    expect(parseSearch(stringifySearch(search))).toEqual(search);
  });
});
