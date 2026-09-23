import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

// Tauri APIs are not available under jsdom; tests that exercise them mock
// `invoke` explicitly, but a default no-op keeps imports from throwing.
vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(() => Promise.resolve(undefined)),
}));

afterEach(() => {
    cleanup();
    localStorage.clear();
});
