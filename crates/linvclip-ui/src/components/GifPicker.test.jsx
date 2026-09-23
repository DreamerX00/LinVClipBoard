import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { I18nProvider } from "../i18n/index.jsx";
import en from "../i18n/en.json";
import pt from "../i18n/pt.json";
import ja from "../i18n/japanese.json";
import hi from "../i18n/hin.json";
import GifPicker from "./GifPicker.jsx";

const DEBOUNCE_MS = 300;

/* ── helpers ─────────────────────────────────────────────────────────── */

function deferred() {
    let resolve, reject;
    const promise = new Promise((res, rej) => {
        resolve = res;
        reject = rej;
    });
    return { promise, resolve, reject };
}

function gifResult(page, { count = 3, hasNext = false } = {}) {
    return {
        page,
        has_next: hasNext,
        items: Array.from({ length: count }, (_, i) => ({
            id: `${page}-${i}`,
            slug: `slug-${page}-${i}`,
            title: `gif ${page}-${i}`,
            preview_url: `https://example.test/p${page}-${i}.webp`,
            gif_url: `https://example.test/g${page}-${i}.gif`,
            width: 100,
            height: 100,
        })),
    };
}

const CATEGORIES = [
    { category: "Cats", query: "cats", preview_url: "https://example.test/cats.webp" },
    { category: "Dogs", query: "dogs", preview_url: "https://example.test/dogs.webp" },
];

const fetchGifCalls = () => invoke.mock.calls.filter(([cmd]) => cmd === "fetch_gifs");
const categoryCalls = () => invoke.mock.calls.filter(([cmd]) => cmd === "fetch_gif_categories");
const spinners = () => document.querySelectorAll(".gif-spinner");

function Wrapper({ searchQuery }) {
    return (
        <I18nProvider>
            <GifPicker searchQuery={searchQuery} onToast={() => {}} />
        </I18nProvider>
    );
}

function renderPicker(searchQuery = "") {
    const utils = render(<Wrapper searchQuery={searchQuery} />);
    return { ...utils, setQuery: (q) => utils.rerender(<Wrapper searchQuery={q} />) };
}

/** Advance fake timers and flush pending promises/react updates. */
async function tick(ms = 0) {
    await act(async () => {
        await vi.advanceTimersByTimeAsync(ms);
    });
}

/* IntersectionObserver stub: records instances so tests can fire entries. */
let observers = [];
class FakeIntersectionObserver {
    constructor(cb) {
        this.cb = cb;
        observers.push(this);
    }
    observe() {}
    disconnect() {
        observers = observers.filter((o) => o !== this);
    }
}
async function intersectSentinel() {
    const latest = observers[observers.length - 1];
    expect(latest, "an IntersectionObserver should be attached").toBeDefined();
    await act(async () => {
        latest.cb([{ isIntersecting: true }]);
    });
}

/* ── setup ───────────────────────────────────────────────────────────── */

beforeEach(() => {
    vi.useFakeTimers();
    observers = [];
    globalThis.IntersectionObserver = FakeIntersectionObserver;
    invoke.mockReset();
    // Default: categories succeed, gifs succeed with one page.
    invoke.mockImplementation((cmd, args) => {
        if (cmd === "fetch_gif_categories") return Promise.resolve(CATEGORIES);
        if (cmd === "fetch_gifs") return Promise.resolve(gifResult(args.page));
        return Promise.resolve(undefined);
    });
});

afterEach(() => {
    vi.useRealTimers();
    delete globalThis.IntersectionObserver;
});

/* ── tests ───────────────────────────────────────────────────────────── */

describe("GifPicker – debounced search", () => {
    it("(a) makes exactly one fetch_gifs call per debounced query", async () => {
        const { setQuery } = renderPicker("c");
        setQuery("ca");
        setQuery("cat");

        await tick(DEBOUNCE_MS - 1);
        expect(fetchGifCalls()).toHaveLength(0);

        await tick(1);
        expect(fetchGifCalls()).toHaveLength(1);
        expect(fetchGifCalls()[0][1]).toEqual({ query: "cat", page: 1, perPage: 24 });

        // Nothing else should fire while the query is unchanged.
        await tick(5_000);
        expect(fetchGifCalls()).toHaveLength(1);
        expect(screen.getAllByRole("button", { name: /gif 1-/ })).toHaveLength(3);
    });

    it("(b) does not refetch when loading toggles", async () => {
        const first = deferred();
        invoke.mockImplementation((cmd) => {
            if (cmd === "fetch_gif_categories") return Promise.resolve(CATEGORIES);
            if (cmd === "fetch_gifs") return first.promise;
            return Promise.resolve(undefined);
        });

        renderPicker("dogs");
        await tick(DEBOUNCE_MS);
        expect(fetchGifCalls()).toHaveLength(1);
        expect(spinners()).toHaveLength(1); // loading = true

        // loading flips true → false here
        await act(async () => {
            first.resolve(gifResult(1));
        });
        await tick(0);
        expect(spinners()).toHaveLength(0);
        expect(screen.getAllByRole("button", { name: /gif 1-/ })).toHaveLength(3);

        // The old bug re-armed the 300 ms timer on every loading change.
        await tick(DEBOUNCE_MS * 10);
        expect(fetchGifCalls()).toHaveLength(1);
    });

    it("ignores a stale response from a superseded query", async () => {
        const slow = deferred();
        const fast = deferred();
        invoke.mockImplementation((cmd, args) => {
            if (cmd === "fetch_gif_categories") return Promise.resolve(CATEGORIES);
            if (cmd === "fetch_gifs") return args.query === "aaa" ? slow.promise : fast.promise;
            return Promise.resolve(undefined);
        });

        const { setQuery } = renderPicker("aaa");
        await tick(DEBOUNCE_MS);
        setQuery("bbb");
        await tick(DEBOUNCE_MS);
        expect(fetchGifCalls().map(([, a]) => a.query)).toEqual(["aaa", "bbb"]);

        await act(async () => {
            fast.resolve({ ...gifResult(1, { count: 2 }), items: [{ id: "b", title: "from bbb", preview_url: "p", gif_url: "g" }] });
        });
        await act(async () => {
            slow.resolve({ ...gifResult(1), items: [{ id: "a", title: "from aaa", preview_url: "p", gif_url: "g" }] });
        });
        await tick(0);

        expect(screen.getByRole("button", { name: "from bbb" })).toBeInTheDocument();
        expect(screen.queryByRole("button", { name: "from aaa" })).not.toBeInTheDocument();
        expect(spinners()).toHaveLength(0);
    });

    it("shows a localized message (not the raw code) when no API key is configured", async () => {
        invoke.mockImplementation((cmd) => {
            if (cmd === "fetch_gif_categories") return Promise.reject("gif_api_key_missing");
            if (cmd === "fetch_gifs") return Promise.reject("gif_api_key_missing");
            return Promise.resolve(undefined);
        });

        const { setQuery } = renderPicker("");
        await tick(0);
        // Categories view: the friendly text alone, no "Couldn't load" headline
        const alert = screen.getByRole("alert");
        expect(alert).toHaveTextContent(en.gif.api_key_missing);
        expect(alert).not.toHaveTextContent(en.gif.categories_failed);
        expect(screen.queryByText(/gif_api_key_missing/)).not.toBeInTheDocument();
        expect(spinners()).toHaveLength(0);
        // The key comes from gif-provider.json at runtime, so it can be fixed
        // upstream while the app is running: Retry must be offered.
        expect(screen.getByRole("button", { name: en.gif.retry })).toBeInTheDocument();

        // Results view
        setQuery("hello");
        await tick(DEBOUNCE_MS);
        expect(fetchGifCalls()).toHaveLength(1);
        expect(screen.getByRole("alert")).toHaveTextContent(en.gif.api_key_missing);
        expect(screen.queryByText(/gif_api_key_missing/)).not.toBeInTheDocument();
        expect(spinners()).toHaveLength(0);
        await tick(5_000);
        expect(fetchGifCalls()).toHaveLength(1); // no storm
    });
});

describe("GifPicker – categories", () => {
    it.each([
        ["gif_api_key_invalid", en.gif.api_key_invalid],
        ["gif_network_error", en.gif.network_error],
    ])("maps %s to its localized message with a Retry button", async (code, message) => {
        let fail = true;
        invoke.mockImplementation((cmd) => {
            if (cmd === "fetch_gif_categories") {
                return fail ? Promise.reject(code) : Promise.resolve(CATEGORIES);
            }
            return Promise.resolve(undefined);
        });

        renderPicker("");
        await tick(0);

        const alert = screen.getByRole("alert");
        expect(alert).toHaveTextContent(message);
        expect(alert).not.toHaveTextContent(code);
        expect(alert).not.toHaveTextContent(en.gif.categories_failed);
        expect(spinners()).toHaveLength(0);

        // e.g. the maintainer rotated the key in gif-provider.json meanwhile
        fail = false;
        fireEvent.click(screen.getByRole("button", { name: en.gif.retry }));
        await tick(0);
        expect(categoryCalls()).toHaveLength(2);
        expect(screen.getByRole("button", { name: "Cats" })).toBeInTheDocument();
    });

    it("(c) shows an error with Retry, not a spinner, when categories fail; Retry refetches", async () => {
        let fail = true;
        invoke.mockImplementation((cmd) => {
            if (cmd === "fetch_gif_categories") {
                return fail ? Promise.reject("API error: 503 Service Unavailable") : Promise.resolve(CATEGORIES);
            }
            return Promise.resolve(undefined);
        });

        renderPicker("");
        expect(spinners()).toHaveLength(1); // initial load
        await tick(0);

        expect(categoryCalls()).toHaveLength(1);
        const alert = screen.getByRole("alert");
        expect(alert).toHaveTextContent(en.gif.categories_failed);
        expect(alert).toHaveTextContent("503");
        expect(spinners()).toHaveLength(0);

        fail = false;
        fireEvent.click(screen.getByRole("button", { name: en.gif.retry }));
        await tick(0);

        expect(categoryCalls()).toHaveLength(2);
        expect(screen.queryByRole("alert")).not.toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Cats" })).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Dogs" })).toBeInTheDocument();
    });

    it("fetches categories once on mount and shows tiles", async () => {
        renderPicker("");
        await tick(0);
        expect(categoryCalls()).toHaveLength(1);
        expect(screen.getByRole("button", { name: "Cats" })).toBeInTheDocument();
        await tick(5_000);
        expect(categoryCalls()).toHaveLength(1);
        expect(fetchGifCalls()).toHaveLength(0); // no results fetch on the home view
    });

    it("clicking a category fetches page 1 for that query once", async () => {
        renderPicker("");
        await tick(0);
        fireEvent.click(screen.getByRole("button", { name: "Cats" }));
        await tick(DEBOUNCE_MS);
        expect(fetchGifCalls()).toHaveLength(1);
        expect(fetchGifCalls()[0][1]).toEqual({ query: "cats", page: 1, perPage: 24 });
        await tick(5_000);
        expect(fetchGifCalls()).toHaveLength(1);
    });
});

describe("GifPicker – infinite scroll", () => {
    it("appends the next page when the sentinel intersects, one request at a time", async () => {
        const page2 = deferred();
        invoke.mockImplementation((cmd, args) => {
            if (cmd === "fetch_gif_categories") return Promise.resolve(CATEGORIES);
            if (cmd === "fetch_gifs") {
                if (args.page === 1) return Promise.resolve(gifResult(1, { hasNext: true }));
                if (args.page === 2) return page2.promise;
                return Promise.resolve(gifResult(args.page));
            }
            return Promise.resolve(undefined);
        });

        renderPicker("cats");
        await tick(DEBOUNCE_MS);
        expect(fetchGifCalls()).toHaveLength(1);
        expect(screen.getAllByRole("button", { name: /^gif / })).toHaveLength(3);

        await intersectSentinel();
        await intersectSentinel(); // second intersect while page 2 is in flight → ignored
        expect(fetchGifCalls()).toHaveLength(2);
        expect(fetchGifCalls()[1][1]).toEqual({ query: "cats", page: 2, perPage: 24 });

        await act(async () => {
            page2.resolve(gifResult(2, { hasNext: false }));
        });
        await tick(0);
        expect(screen.getAllByRole("button", { name: /^gif / })).toHaveLength(6);
        expect(screen.getByRole("button", { name: "gif 2-0" })).toBeInTheDocument();
        await tick(5_000);
        expect(fetchGifCalls()).toHaveLength(2);
    });
});

describe("GIF i18n keys", () => {
    it("exist in every locale", () => {
        for (const [name, locale] of Object.entries({ en, pt, ja, hi })) {
            for (const key of ["retry", "api_key_missing", "api_key_invalid", "network_error", "categories_failed", "no_results"]) {
                expect(locale.gif?.[key], `${name}.gif.${key}`).toEqual(expect.any(String));
                expect(locale.gif[key].length, `${name}.gif.${key}`).toBeGreaterThan(0);
            }
        }
    });
});
