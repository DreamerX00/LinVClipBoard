import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/index.jsx";

const PER_PAGE = 24;

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

function GifPicker({ searchQuery, onToast }) {
    const { t } = useTranslation();
    const [gifs, setGifs] = useState([]);
    const [categories, setCategories] = useState([]);
    const [activeCategory, setActiveCategory] = useState(null);
    const [loading, setLoading] = useState(false);
    const [page, setPage] = useState(1);
    const [hasNext, setHasNext] = useState(false);
    const [error, setError] = useState(null);
    const lastQuery = useRef("");
    const scrollRef = useRef(null);
    const sentinelRef = useRef(null);

    const showCategories = !searchQuery && !activeCategory;

    // ── Fetch categories on mount ──
    useEffect(() => {
        let cancelled = false;
        (async () => {
            try {
                const cats = await invoke("fetch_gif_categories");
                if (!cancelled) setCategories(cats);
            } catch (_) {}
        })();
        return () => { cancelled = true; };
    }, []);

    // ── Fetch GIFs ──
    const fetchGifs = useCallback(
        async (resetPage = false) => {
            if (loading) return;
            setLoading(true);
            setError(null);
            try {
                const nextPage = resetPage ? 1 : page + 1;
                const query = searchQuery || activeCategory || "";
                const result = await invoke("fetch_gifs", {
                    query,
                    page: nextPage,
                    perPage: PER_PAGE,
                });
                if (resetPage) {
                    setGifs(result.items);
                    setPage(1);
                } else {
                    setGifs((prev) => [...prev, ...result.items]);
                    setPage(nextPage);
                }
                setHasNext(result.has_next);
            } catch (err) {
                setError(String(err));
            } finally {
                setLoading(false);
            }
        },
        [searchQuery, activeCategory, page, loading]
    );

    // Reset on query change
    useEffect(() => {
        if (searchQuery !== lastQuery.current) {
            lastQuery.current = searchQuery;
            setGifs([]);
            setPage(1);
            setHasNext(false);
            setError(null);
            if (searchQuery) setActiveCategory(null);
        }
    }, [searchQuery]);

    // Trigger fetch with debounce
    useEffect(() => {
        if (showCategories) return;
        const timer = setTimeout(() => fetchGifs(true), 300);
        return () => clearTimeout(timer);
    }, [searchQuery, activeCategory, fetchGifs]);

    // Infinite scroll — use ref to avoid stale closure
    const fetchGifsRef = useRef(fetchGifs);
    useEffect(() => { fetchGifsRef.current = fetchGifs; }, [fetchGifs]);

    useEffect(() => {
        const sentinel = sentinelRef.current;
        if (!sentinel) return;
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
    }, [hasNext, loading]);

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
        setActiveCategory(query);
        setGifs([]);
        setPage(1);
        setHasNext(false);
        setError(null);
    }, []);

    const handleBack = useCallback(() => {
        setActiveCategory(null);
        setGifs([]);
        setPage(1);
        setHasNext(false);
        setError(null);
    }, []);

    // ── Categories (home) view ──
    if (showCategories) {
        return (
            <div className="picker-scroll" ref={scrollRef}>
                {categories.length > 0 ? (
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
                ) : (
                    <div className="gif-loading"><span className="gif-spinner" /></div>
                )}
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

            {error && <div className="gif-error" role="alert">⚠️ {error}</div>}

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
