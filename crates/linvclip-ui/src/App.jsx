import { useState, useEffect, useCallback, useRef, lazy, Suspense } from "react";
import { createPortal } from "react-dom";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "./i18n/index.jsx";
import { useZoom } from "./hooks/useZoom.js";
import AppHeader from "./components/AppHeader";
import TabBar from "./components/TabBar";
import SearchBar from "./components/SearchBar";
import ClipboardList from "./components/ClipboardList";
import EmojiPicker from "./components/EmojiPicker";
import SymbolPicker from "./components/SymbolPicker";
import GifPicker from "./components/GifPicker";
import FilterPills from "./components/FilterPills";
import Footer from "./components/Footer";
import SettingsPanel from "./components/SettingsPanel";
import ConfirmDialog from "./components/ConfirmDialog";
import ContextMenu from "./components/ContextMenu";
import QrModal from "./components/QrModal";
const PreviewPane = lazy(() => import("./components/PreviewPane"));
import SelectionBar from "./components/SelectionBar";
import SnippetPicker from "./components/SnippetPicker";
import ColorPicker from "./components/ColorPicker";
import CheatSheet from "./components/CheatSheet";
import { useKeybindings } from "./contexts/KeybindingContext.jsx";

function App() {
    const { t } = useTranslation();
    const { zoom, setZoom, zoomIn, zoomOut, zoomReset } = useZoom();
    const { resolveAction, vimMode, vimState, setVimState, handleSearchFocus, handleSearchBlur } = useKeybindings();
    const [showCheatSheet, setShowCheatSheet] = useState(false);
    const [activeTab, setActiveTab] = useState("clipboard");
    const [filterType, setFilterType] = useState("all");
    const [items, setItems] = useState([]);
    const [total, setTotal] = useState(0);
    const [searchQuery, setSearchQuery] = useState("");
    const [isRegex, setIsRegex] = useState(false);
    const [loading, setLoading] = useState(false);
    const loadingRef = useRef(false);
    const [status, setStatus] = useState(null);
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [toast, setToast] = useState(null);
    const [showSettings, setShowSettings] = useState(false);
    const [showConfirm, setShowConfirm] = useState(false);
    const [theme, setTheme] = useState(() => {
        const stored = localStorage.getItem("theme");
        if (stored) return stored;
        return window.matchMedia?.("(prefers-color-scheme: dark)")?.matches ? "dark" : "light";
    });
    const [ctxMenu, setCtxMenu] = useState(null); // { item, x, y }
    const [qrText, setQrText] = useState(null); // text to generate QR for
    const [snippetInitContent, setSnippetInitContent] = useState(null);
    const [showPreview, setShowPreview] = useState(() => localStorage.getItem("showPreview") === "true");
    const [previewItem, setPreviewItem] = useState(null);
    const [selectedIds, setSelectedIds] = useState(new Set());
    const [appConfig, setAppConfig] = useState(null);
    const offset = useRef(0);
    const LIMIT = 30;

    // Resolve "auto" theme based on OS preference
    const resolveTheme = (t) => {
        if (t !== "auto") return t;
        return window.matchMedia?.("(prefers-color-scheme: light)")?.matches
            ? "catppuccin-latte"
            : "catppuccin-mocha";
    };

    // Apply theme + accent to DOM
    useEffect(() => {
        document.documentElement.setAttribute("data-theme", resolveTheme(theme));
        localStorage.setItem("theme", theme);
        // Set --list-width CSS var from stored window width
        const baseW = parseInt(localStorage.getItem("windowWidth")) || 420;
        document.documentElement.style.setProperty("--list-width", baseW + "px");
    }, [theme]);

    // Apply accent color override from localStorage
    useEffect(() => {
        const accent = localStorage.getItem("accent_color") || "auto";
        if (accent !== "auto") {
            document.documentElement.style.setProperty("--accent", accent);
            // Compute lighter/darker for hover/active
            document.documentElement.style.setProperty("--accent-hover", accent + "cc");
            document.documentElement.style.setProperty("--accent-active", accent);
            document.documentElement.style.setProperty("--accent-glow", accent + "4d");
            document.documentElement.style.setProperty("--border-focus", accent + "80");
        } else {
            document.documentElement.style.removeProperty("--accent");
            document.documentElement.style.removeProperty("--accent-hover");
            document.documentElement.style.removeProperty("--accent-active");
            document.documentElement.style.removeProperty("--accent-glow");
            document.documentElement.style.removeProperty("--border-focus");
        }
    }, [theme]); // re-apply when theme changes

    // Listen for OS theme changes when in "auto" mode
    useEffect(() => {
        const mq = window.matchMedia?.("(prefers-color-scheme: light)");
        if (!mq) return;
        const handler = () => {
            if (localStorage.getItem("theme") === "auto") {
                document.documentElement.setAttribute("data-theme", resolveTheme("auto"));
            }
        };
        mq.addEventListener("change", handler);
        return () => mq.removeEventListener("change", handler);
    }, []);

    // Store latest values in refs so the document-level keydown handler
    // always sees current state without needing to re-attach the listener.
    const itemsRef = useRef(items);
    const selectedIndexRef = useRef(selectedIndex);
    const searchQueryRef = useRef(searchQuery);
    useEffect(() => { itemsRef.current = items; }, [items]);
    useEffect(() => { selectedIndexRef.current = selectedIndex; }, [selectedIndex]);
    useEffect(() => { searchQueryRef.current = searchQuery; }, [searchQuery]);

    const fetchItems = useCallback(
        async (reset = false, preserveLength = false) => {
            if (loadingRef.current) return;
            loadingRef.current = true;
            // Don't show loading spinner for background auto-refreshes
            if (!preserveLength) setLoading(true);
            try {
                let currentOffset = offset.current;
                let fetchLimit = LIMIT;

                if (reset) {
                    if (preserveLength && itemsRef.current.length > LIMIT) {
                        // Background refresh: preserve all loaded items
                        fetchLimit = itemsRef.current.length;
                        currentOffset = 0;
                    } else {
                        currentOffset = 0;
                        offset.current = 0;
                    }
                }

                let result;
                if (searchQuery.trim()) {
                    if (isRegex) {
                        result = await invoke("search_items_regex", {
                            pattern: searchQuery,
                            limit: fetchLimit,
                            offset: currentOffset,
                        });
                    } else {
                        result = await invoke("search_items", {
                            query: searchQuery,
                            limit: fetchLimit,
                            offset: currentOffset,
                        });
                    }
                } else {
                    result = await invoke("get_items", {
                        offset: currentOffset,
                        limit: fetchLimit,
                    });
                }

                if (result) {
                    if (reset) {
                        setItems(result.items || []);
                        if (preserveLength) {
                            offset.current = (result.items || []).length;
                        } else {
                            offset.current = fetchLimit;
                        }
                    } else {
                        setItems((prev) => {
                            const newItems = result.items || [];
                            const seen = new Set(prev.map(i => i.id));
                            const additions = newItems.filter(i => !seen.has(i.id));
                            return [...prev, ...additions];
                        });
                        offset.current += fetchLimit;
                    }
                    setTotal(result.total || 0);
                }
            } catch (err) {
                console.error("Failed to fetch items:", err);
            } finally {
                loadingRef.current = false;
                setLoading(false);
            }
        },
        [searchQuery, isRegex]
    );

    const fetchItemsRef = useRef(fetchItems);
    useEffect(() => { fetchItemsRef.current = fetchItems; }, [fetchItems]);

    // Initial load
    // Load app config for smart features
    useEffect(() => {
        invoke("get_config").then(setAppConfig).catch(() => {});
    }, []);

    useEffect(() => {
        fetchItems(true);
        fetchStatus();

        // Listen for push-based clipboard-updated events from the daemon (#13)
        let unlisten;
        const setupListener = async () => {
            unlisten = await listen("clipboard-updated", () => {
                if (!searchQueryRef.current.trim()) {
                    fetchItemsRef.current(true, true);
                }
                fetchStatus();
            });
        };
        setupListener();

        // Fallback polling every 5s in case events are missed
        const interval = setInterval(() => {
            if (!searchQueryRef.current.trim()) {
                fetchItemsRef.current(true, true);
            }
            fetchStatus();
        }, 5000);

        return () => {
            clearInterval(interval);
            if (unlisten) unlisten();
        };
    }, []);

    // Re-fetch on search change or regex toggle
    useEffect(() => {
        const timer = setTimeout(() => {
            fetchItems(true);
        }, 300);
        return () => clearTimeout(timer);
    }, [searchQuery, isRegex]);

    const fetchStatus = async () => {
        try {
            const s = await invoke("get_status");
            setStatus(s);
        } catch (err) {
            console.error("Status fetch failed:", err);
        }
    };

    const toastTimer = useRef(null);
    const showToast = (message) => {
        setToast(message);
        if (toastTimer.current) clearTimeout(toastTimer.current);
        toastTimer.current = setTimeout(() => setToast(null), 700);
    };

    const hideWindow = async () => {
        try {
            const win = getCurrentWindow();
            await win.hide();
        } catch (err) {
            console.error("Hide failed:", err);
        }
    };

    const pasteTimer = useRef(null);
    const handlePaste = async (id) => {
        try {
            await invoke("paste_item", { id });
            showToast("✅ Copied!");
            // Auto-close after showing feedback
            if (pasteTimer.current) clearTimeout(pasteTimer.current);
            pasteTimer.current = setTimeout(() => hideWindow(), 500);
        } catch (err) {
            showToast("❌ Failed");
            console.error("Paste failed:", err);
        }
    };

    const handlePin = async (id) => {
        try {
            await invoke("pin_item", { id });
            setItems((prev) =>
                prev.map((item) =>
                    item.id === id ? { ...item, pinned: !item.pinned } : item
                )
            );
        } catch (err) {
            console.error("Pin failed:", err);
        }
    };

    const handleDelete = async (id) => {
        try {
            await invoke("delete_item", { id });
            setItems((prev) => prev.filter((item) => item.id !== id));
            setTotal((t) => Math.max(0, t - 1));
        } catch (err) {
            console.error("Delete failed:", err);
        }
    };

    const handleLoadMore = () => {
        if (items.length < total) {
            fetchItems(false);
        }
    };

    const handleClearAll = async () => {
        // Actual clear logic (called after confirmation)
        try {
            await invoke("clear_all");
            showToast("🗑️ Cleared!");
            fetchItems(true);
            fetchStatus();
        } catch (err) {
            // Fallback: delete items one by one
            const unpinned = items.filter(i => !i.pinned);
            for (const item of unpinned) {
                try {
                    await invoke("delete_item", { id: item.id });
                } catch (_) { }
            }
            showToast("🗑️ Cleared!");
            fetchItems(true);
            fetchStatus();
        }
    };

    // Triggered by StatusBar — shows confirmation first (#41)
    const requestClearAll = () => {
        setShowConfirm(true);
    };

    const handleConfirmClear = () => {
        setShowConfirm(false);
        handleClearAll();
    };

    // Context menu handler
    const handleContextMenu = useCallback((e, item) => {
        setCtxMenu({ item, x: e.clientX, y: e.clientY });
    }, []);

    // Update a single item in the list (after tag add/remove)
    const handleItemUpdate = useCallback((updated) => {
        setItems((prev) => prev.map((it) => (it.id === updated.id ? updated : it)));
    }, []);

    // Persist preview toggle & auto-resize window
    useEffect(() => {
        localStorage.setItem("showPreview", showPreview);
        // Small delay on initial render to let Tauri window fully initialize
        const timer = setTimeout(async () => {
            try {
                const { LogicalSize } = await import("@tauri-apps/api/dpi");
                const win = getCurrentWindow();
                const baseW = parseInt(localStorage.getItem("windowWidth")) || 420;
                const baseH = parseInt(localStorage.getItem("windowHeight")) || 520;
                const previewW = parseInt(localStorage.getItem("previewWidth")) || 380;
                document.documentElement.style.setProperty("--list-width", baseW + "px");

                // Try to get current height, fallback to stored height
                let h = baseH;
                try {
                    const factor = await win.scaleFactor();
                    const size = await win.innerSize();
                    h = Math.round(size.height / factor);
                } catch (_) {}

                const totalW = showPreview ? baseW + previewW : baseW;
                await win.setSize(new LogicalSize(totalW, h));
            } catch (err) {
                console.error("Preview resize failed:", err);
            }
        }, 150); // 150ms delay ensures window is ready on first mount
        return () => clearTimeout(timer);
    }, [showPreview]);

    // Update preview item when selection changes
    useEffect(() => {
        if (showPreview && filteredItems[selectedIndex]) {
            setPreviewItem(filteredItems[selectedIndex]);
        }
    }, [selectedIndex, showPreview, items, filterType]);

    // Multi-select handlers
    const handleSelectToggle = useCallback((id) => {
        setSelectedIds((prev) => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            return next;
        });
    }, []);

    const handleSelectRange = useCallback((toIndex) => {
        // Select from selectedIndex to toIndex inclusive
        const from = Math.min(selectedIndex, toIndex);
        const to = Math.max(selectedIndex, toIndex);
        setSelectedIds((prev) => {
            const next = new Set(prev);
            for (let i = from; i <= to; i++) {
                if (filteredItems[i]) next.add(filteredItems[i].id);
            }
            return next;
        });
    }, [selectedIndex, items, filterType]);

    const clearSelection = useCallback(() => setSelectedIds(new Set()), []);

    // Store selectedIds in ref for keyboard handler
    const selectedIdsRef = useRef(selectedIds);
    useEffect(() => { selectedIdsRef.current = selectedIds; }, [selectedIds]);

    // ─── Global keyboard handler via keybinding context ───
    // Uses refs so the handler always sees fresh state without re-attaching.
    const showSettingsRef = useRef(showSettings);
    const showCheatSheetRef = useRef(showCheatSheet);
    const showConfirmRef = useRef(showConfirm);
    const qrTextRef = useRef(qrText);
    const ctxMenuRef = useRef(ctxMenu);
    useEffect(() => { showSettingsRef.current = showSettings; }, [showSettings]);
    useEffect(() => { showCheatSheetRef.current = showCheatSheet; }, [showCheatSheet]);
    useEffect(() => { showConfirmRef.current = showConfirm; }, [showConfirm]);
    useEffect(() => { qrTextRef.current = qrText; }, [qrText]);
    useEffect(() => { ctxMenuRef.current = ctxMenu; }, [ctxMenu]);

    useEffect(() => {
        const handleKeyDown = (e) => {
            // Don't intercept keys when modals are open
            if (showCheatSheetRef.current || showSettingsRef.current || showConfirmRef.current || qrTextRef.current || ctxMenuRef.current) return;

            const result = resolveAction(e);
            if (!result) return;

            const { action, preventDefault } = result;
            if (preventDefault) {
                e.preventDefault();
                e.stopPropagation();
            }

            const curItems = itemsRef.current;
            const curIdx = selectedIndexRef.current;

            switch (action) {
                case "close_window":
                case "exit_search":
                    if (selectedIdsRef.current.size > 0) {
                        setSelectedIds(new Set());
                    } else if (document.activeElement?.tagName === "INPUT") {
                        document.activeElement.blur();
                        setVimState("normal");
                    } else {
                        hideWindow();
                    }
                    break;
                case "select_next":
                    setSelectedIndex((i) => Math.min(i + 1, curItems.length - 1));
                    break;
                case "select_prev":
                    setSelectedIndex((i) => Math.max(i - 1, 0));
                    break;
                case "select_first":
                    setSelectedIndex(0);
                    break;
                case "select_last":
                    setSelectedIndex(Math.max(0, curItems.length - 1));
                    break;
                case "page_down":
                    setSelectedIndex((i) => Math.min(i + 10, curItems.length - 1));
                    break;
                case "page_up":
                    setSelectedIndex((i) => Math.max(i - 10, 0));
                    break;
                case "paste_selected":
                    if (curItems[curIdx]) handlePaste(curItems[curIdx].id);
                    break;
                case "delete_selected":
                    if (curItems[curIdx]) handleDelete(curItems[curIdx].id);
                    break;
                case "pin_selected":
                    if (curItems[curIdx]) handlePin(curItems[curIdx].id);
                    break;
                case "copy_selected":
                    if (curItems[curIdx]) handlePaste(curItems[curIdx].id);
                    break;
                case "search_focus": {
                    const input = document.querySelector(".search-input");
                    if (input) { input.focus(); setVimState("insert"); }
                    break;
                }
                case "clear_search":
                    setSearchQuery("");
                    break;
                case "toggle_preview":
                    setShowPreview((p) => !p);
                    break;
                case "close_preview":
                    setShowPreview(false);
                    break;
                case "context_menu":
                    if (curItems[curIdx]) {
                        // Simulate context menu at item position
                        const el = document.querySelector(".clip-item.selected");
                        if (el) {
                            const rect = el.getBoundingClientRect();
                            setCtxMenu({ item: curItems[curIdx], x: rect.right - 20, y: rect.top + 10 });
                        }
                    }
                    break;
                case "select_all": {
                    const allIds = new Set(curItems.map((it) => it.id));
                    setSelectedIds(allIds);
                    break;
                }
                case "tab_clipboard": setActiveTab("clipboard"); break;
                case "tab_emojis": setActiveTab("emojis"); break;
                case "tab_symbols": setActiveTab("symbols"); break;
                case "tab_gifs": setActiveTab("gifs"); break;
                case "tab_snippets": setActiveTab("snippets"); break;
                case "tab_colors": setActiveTab("colors"); break;
                case "show_cheatsheet":
                    setShowCheatSheet(true);
                    break;
                case "vim_toggle_select":
                    if (curItems[curIdx]) {
                        const id = curItems[curIdx].id;
                        setSelectedIds((prev) => {
                            const next = new Set(prev);
                            if (next.has(id)) next.delete(id); else next.add(id);
                            return next;
                        });
                    }
                    break;
                case "zoom_in": zoomIn(); break;
                case "zoom_out": zoomOut(); break;
                case "zoom_reset": zoomReset(); break;
            }
        };

        document.addEventListener("keydown", handleKeyDown, true);
        return () => document.removeEventListener("keydown", handleKeyDown, true);
    }, [resolveAction, zoomIn, zoomOut, zoomReset]);

    // Clear search when switching tabs
    const handleTabChange = (tab) => {
        setActiveTab(tab);
        setSearchQuery("");
        if (tab !== "clipboard") setFilterType("all");
    };

    // Client-side filtering for clipboard items
    const filteredItems = items.filter((item) => {
        if (filterType === "all") return true;
        if (filterType === "pinned") return item.pinned;
        if (filterType === "text") return item.content_type === "plain_text";
        if (filterType === "image") return item.content_type === "image";
        if (filterType === "html") return item.content_type === "html";
        if (filterType === "uri") return item.content_type === "uri";
        if (filterType === "files") return item.content_type === "files";
        return true;
    });

    // Determine search placeholder based on active tab
    const searchPlaceholder =
        activeTab === "emojis"
            ? t("search.placeholder_emojis")
            : activeTab === "symbols"
            ? t("search.placeholder_symbols")
            : activeTab === "gifs"
            ? t("search.placeholder_gifs")
            : t("search.placeholder_clipboard");

    return (
        <div className={`app${showPreview ? " app-split" : ""}`} role="application" aria-label={t("app.title")}>
            <div className="app-container">
                <AppHeader
                    onOpenSettings={() => setShowSettings(true)}
                    onClearAll={requestClearAll}
                    showPreview={showPreview}
                    onTogglePreview={() => setShowPreview((p) => !p)}
                />

                <TabBar activeTab={activeTab} onTabChange={handleTabChange} />

                {activeTab !== "colors" && (
                    <SearchBar
                        value={searchQuery}
                        onChange={setSearchQuery}
                        placeholder={searchPlaceholder}
                        onFocus={handleSearchFocus}
                        onBlur={handleSearchBlur}
                        isRegex={isRegex && activeTab === "clipboard"}
                        onToggleRegex={() => setIsRegex((r) => !r)}
                    />
                )}

                <main role="main" aria-label={t("tabs." + activeTab)}>
                    <div className="main-split">
                        <div className="main-list">
                    {activeTab === "clipboard" && (
                        <>
                            <FilterPills activeFilter={filterType} onFilterChange={setFilterType} />
                            <ClipboardList
                                items={filteredItems}
                                selectedIndex={selectedIndex}
                                onPaste={handlePaste}
                                onPin={handlePin}
                                onDelete={handleDelete}
                                onLoadMore={handleLoadMore}
                                loading={loading}
                                hasMore={items.length < total}
                                onContextMenu={handleContextMenu}
                                selectedIds={selectedIds}
                                onSelectToggle={handleSelectToggle}
                                onSelectRange={handleSelectRange}
                                config={appConfig}
                                onToast={showToast}
                                onItemUpdate={handleItemUpdate}
                            />
                            <SelectionBar
                                selectedIds={selectedIds}
                                items={filteredItems}
                                onClearSelection={clearSelection}
                                onItemsChanged={() => fetchItems(true)}
                                onToast={showToast}
                            />
                        </>
                    )}
                    {activeTab === "emojis" && (
                        <EmojiPicker searchQuery={searchQuery} onToast={showToast} />
                    )}
                    {activeTab === "symbols" && (
                        <SymbolPicker searchQuery={searchQuery} onToast={showToast} />
                    )}
                    {activeTab === "gifs" && (
                        <GifPicker searchQuery={searchQuery} onToast={showToast} />
                    )}
                    {activeTab === "snippets" && (
                        <SnippetPicker searchQuery={searchQuery} onToast={showToast} initialContent={snippetInitContent} onConsumeInitContent={() => setSnippetInitContent(null)} />
                    )}
                    {activeTab === "colors" && (
                        <ColorPicker onToast={showToast} />
                    )}
                        </div>
                        {showPreview && activeTab === "clipboard" && (
                            <Suspense fallback={<div className="preview-pane-loading">Loading…</div>}>
                                <PreviewPane
                                    item={previewItem}
                                    onPaste={handlePaste}
                                    onToast={showToast}
                                    onItemUpdate={handleItemUpdate}
                                />
                            </Suspense>
                        )}
                    </div>
                </main>

                {vimMode && (
                    <div className={`vim-mode-indicator vim-mode-${vimState}`}>
                        -- {vimState === "insert" ? t("vim.insert_mode") : t("vim.normal_mode")} --
                    </div>
                )}
                <Footer />
            </div>

            {toast && <div className="copy-toast" role="alert">{toast}</div>}

            {showSettings && (
                <SettingsPanel
                    onClose={() => setShowSettings(false)}
                    zoom={zoom}
                    onZoomChange={setZoom}
                />
            )}

            {showConfirm && (
                <ConfirmDialog
                    message={t("confirm.clear_all")}
                    onConfirm={handleConfirmClear}
                    onCancel={() => setShowConfirm(false)}
                />
            )}

            {ctxMenu && createPortal(
                <ContextMenu
                    item={ctxMenu.item}
                    x={ctxMenu.x}
                    y={ctxMenu.y}
                    onClose={() => setCtxMenu(null)}
                    onPin={handlePin}
                    onDelete={handleDelete}
                    onPaste={handlePaste}
                    onToast={showToast}
                    onShowQr={(text) => setQrText(text)}
                    onItemUpdate={handleItemUpdate}
                    onSaveAsSnippet={(item) => {
                        setSnippetInitContent(item.preview_text || item.content || "");
                        setActiveTab("snippets");
                        setCtxMenu(null);
                    }}
                />,
                document.body
            )}

            {qrText && (
                <QrModal
                    text={qrText}
                    onClose={() => setQrText(null)}
                    onToast={showToast}
                />
            )}

            {showCheatSheet && (
                <CheatSheet onClose={() => setShowCheatSheet(false)} />
            )}
        </div>
    );
}

export default App;
