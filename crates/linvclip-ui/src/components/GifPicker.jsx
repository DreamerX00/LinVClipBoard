import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/index.jsx";

const PER_PAGE = 24;
const DEBOUNCE_MS = 300;

/**
 * Error codes returned by the Rust GIF commands that have a friendly,
 * localized message. Anything else is shown verbatim.
 * Keep in sync with `GIF_API_KEY_MISSING` in src-tauri/src/lib.rs.
 */
const GIF_ERROR_KEYS = {
    gif_api_key_missing: "gif.api_key_missing",
};

/** Map a backend error (string or Error) to a user-facing message. */
function describeGifError(err, t) {
    const code = err instanceof Error ? err.message : String(err);
    const key = GIF_ERROR_KEYS[code];
    return { message: key ? t(key) : code, retryable: !key };
}

/* Tiny component that shows a shimmer skeleton until its image loads. */
function GifImage({ src, alt }) {
    const [loaded, setLoaded] = useState(false);
    return (
        <>
            {!loaded && <div className="gif-shimmer" />}
            <img
                src={src}
                alt={alt}
                loading="lazy"
                className={`gif-img ${loaded ? "gif-img--loaded" : "gif-img--loading"}`}
                onLoad={() => setLoaded(true)}
            />
        </>
    );
}

/** Error banner with an optional Retry button. Never renders a spinner. */
function GifError({ error, title, onRetry, t }) {
    return (
        <div className="gif-error" role="alert">
            <p className="gif-error-text">⚠️ {title || error.message}</p>
            {title && <p className="gif-error-detail">{error.message}</p>}
            {error.retryable && onRetry && (
                <button type="button" className="gif-retry-btn" onClick={onRetry}>
                    {t("gif.retry")}
                </button>
            )}
        </div>
    );
}

function GifPicker({ searchQuery, onToast }) {
    const { t } = useTranslation();
    const [gifs, setGifs] = useState([]);
    const [categories, setCategories] = useState([]);
    const [categoriesLoading, setCategoriesLoading] = useState(true);
    const [categoriesError, setCategoriesError] = useState(null);
    const [activeCategory, setActiveCategory] = useState(null);
    const [loading, setLoading] = useState(false);
    const [hasNext, setHasNext] = useState(false);
    const [error, setError] = useState(null);
    const scrollRef = useRef(null);
    const sentinelRef = useRef(null);

    // Request bookkeeping lives in refs so it never feeds back into hook deps:
    //  - pageRef:      last page successfully loaded for the current query
    //  - inFlightRef:  true while a fetch_gifs call is outstanding
    //  - requestIdRef: monotonically increasing id; a response whose id is no
    //                  longer current belongs to a superseded query → ignored
    const pageRef = useRef(1);
    const inFlightRef = useRef(false);
    const requestIdRef = useRef(0);
    const categoriesRequestRef = useRef(0);
    const mountedRef = useRef(true);
    useEffect(() => {
        mountedRef.current = true;
        return () => { mountedRef.current = false; };
    }, []);

    const showCategories = !searchQuery && !activeCategory;

    // ── Categories ──
    const loadCategories = useCallback(async () => {
        const id = ++categoriesRequestRef.current;
        setCategoriesLoading(true);
        setCategoriesError(null);
        try {
            const cats = await invoke("fetch_gif_categories");
            if (!mountedRef.current || id !== categoriesRequestRef.current) return;
            setCategories(Array.isArray(cats) ? cats : []);
        } catch (err) {
            if (!mountedRef.current || id !== categoriesRequestRef.current) return;
            setCategories([]);
            setCategoriesError(describeGifError(err, t));
        } finally {
            if (mountedRef.current && id === categoriesRequestRef.current) {
                setCategoriesLoading(false);
            }
        }
    }, [t]);

    useEffect(() => {
        loadCategories();
    }, [loadCategories]);

    // ── Results ──
    /** Drop current results and invalidate any outstanding fetch. */
    const resetResults = useCallback(() => {
        requestIdRef.current += 1;
        inFlightRef.current = false;
        pageRef.current = 1;
        setGifs([]);
        setHasNext(false);
        setError(null);
        setLoading(false);
    }, []);

    /**
     * Fetch page 1 (`resetPage`) or the next page for the current query.
     * Deps are only the query inputs, so this identity — and therefore the
     * debounce effect below — is stable while a request is in flight.
     */
    const fetchGifs = useCallback(
        async (resetPage = false) => {
            // Pagination never overlaps itself; a new query always supersedes.
            if (!resetPage && inFlightRef.current) return;
            const id = ++requestIdRef.current;
            const nextPage = resetPage ? 1 : pageRef.current + 1;
            const query = searchQuery || activeCategory || "";
            inFlightRef.current = true;
            setLoading(true);
            setError(null);
            try {
                const result = await invoke("fetch_gifs", {
                    query,
                    page: nextPage,
                    perPage: PER_PAGE,
                });
                if (!mountedRef.current || id !== requestIdRef.current) return; // stale
                pageRef.current = nextPage;
                const items = Array.isArray(result?.items) ? result.items : [];
                setGifs((prev) => (resetPage ? items : [...prev, ...items]));
                setHasNext(Boolean(result?.has_next));
            } catch (err) {
                if (!mountedRef.current || id !== requestIdRef.current) return; // stale
                setError(describeGifError(err, t));
            } finally {
                if (mountedRef.current && id === requestIdRef.current) {
                    inFlightRef.current = false;
                    setLoading(false);
                }
            }
        },
        [searchQuery, activeCategory, t]
    );

    // Reset on query change (also drops the active category once typing starts)
    useEffect(() => {
        resetResults();
        if (searchQuery) setActiveCategory(null);
    }, [searchQuery, resetResults]);

    // Single debounced fetch per (query, category) change
    useEffect(() => {
        if (showCategories) return undefined;
        const timer = setTimeout(() => fetchGifs(true), DEBOUNCE_MS);
        return () => clearTimeout(timer);
    }, [showCategories, fetchGifs]);

    // Infinite scroll — use ref to avoid stale closure
    const fetchGifsRef = useRef(fetchGifs);
    useEffect(() => { fetchGifsRef.current = fetchGifs; }, [fetchGifs]);

    useEffect(() => {
        const sentinel = sentinelRef.current;
        if (!sentinel || typeof IntersectionObserver === "undefined") return undefined;
        const observer = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting && hasNext && !loading) {
                    fetchGifsRef.current(false);
                }
            },
            { root: scrollRef.current, threshold: 0.1 }
        );
        observer.observe(sentinel);
        return () => observer.disconnect();
    }, [hasNext, loading, showCategories]);

    const handleCopyGif = useCallback(
        async (gif) => {
            try {
                await invoke("copy_gif", { url: gif.gif_url });
                invoke("register_gif_share", {
                    slug: gif.slug || gif.id,
                    query: searchQuery || activeCategory || "",
                }).catch(() => {});
                if (onToast) onToast("📋 " + t("gif.copied"));
            } catch (_) {
                if (onToast) onToast("❌ " + t("clipboard.copy_failed"));
            }
        },
        [t, onToast, searchQuery, activeCategory]
    );

    const handleCategoryClick = useCallback((query) => {
        resetResults();
        setActiveCategory(query);
    }, [resetResults]);

    const handleBack = useCallback(() => {
        resetResults();
        setActiveCategory(null);
    }, [resetResults]);

    const handleRetryResults = useCallback(() => fetchGifs(true), [fetchGifs]);

    // ── Categories (home) view ──
    if (showCategories) {
        let body;
        if (categoriesError) {
            body = (
                <GifError
                    error={categoriesError}
                    title={categoriesError.retryable ? t("gif.categories_failed") : null}
                    onRetry={loadCategories}
                    t={t}
                />
            );
        } else if (categories.length > 0) {
            body = (
                <div className="gif-categories-grid">
                    {categories.map((cat) => (
                        <button
                            key={cat.query}
                            className="gif-category-tile"
                            onClick={() => handleCategoryClick(cat.query)}
                            aria-label={cat.category}
                        >
                            <GifImage src={cat.preview_url} alt={cat.category} />
                            <span className="gif-category-label">{cat.category}</span>
                        </button>
                    ))}
                </div>
            );
        } else if (categoriesLoading) {
            body = <div className="gif-loading"><span className="gif-spinner" /></div>;
        } else {
            body = <div className="picker-empty"><p>{t("gif.no_results")}</p></div>;
        }
        return (
            <div className="picker-scroll" ref={scrollRef}>
                {body}
                <div className="gif-powered-by">Powered by KLIPY</div>
            </div>
        );
    }

    // ── Search / Category results view ──
    return (
        <div className="picker-scroll" ref={scrollRef}>
            {activeCategory && (
                <div className="gif-results-header">
                    <button className="gif-back-btn" onClick={handleBack} aria-label="Back">←</button>
                    <span className="gif-results-title">{activeCategory}</span>
                </div>
            )}

            {error && <GifError error={error} onRetry={handleRetryResults} t={t} />}

            <div className="gif-grid">
                {gifs.map((gif) => (
                    <button
                        key={gif.id}
                        className="gif-cell"
                        onClick={() => handleCopyGif(gif)}
                        title={gif.title || t("gif.copy_hint")}
                        aria-label={gif.title || "GIF"}
                    >
                        <GifImage src={gif.preview_url} alt={gif.title || "GIF"} />
                    </button>
                ))}
            </div>

            <div ref={sentinelRef} className="gif-sentinel" />

            {loading && <div className="gif-loading"><span className="gif-spinner" /></div>}

            {!loading && gifs.length === 0 && !error && (
                <div className="picker-empty"><p>{t("gif.no_results")}</p></div>
            )}

            {gifs.length > 0 && <div className="gif-powered-by">Powered by KLIPY</div>}
        </div>
    );
}

export default GifPicker;
