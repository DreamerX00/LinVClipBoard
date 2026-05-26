import { createContext, useContext, useState, useEffect, useCallback, useRef } from "react";

/**
 * Default keybinding map: action → key combo string.
 * Key combos follow the format: "Ctrl+Shift+k", "ArrowDown", "Enter", etc.
 */
const DEFAULT_BINDINGS = {
    // Navigation
    select_next: "ArrowDown",
    select_prev: "ArrowUp",
    select_first: "Home",
    select_last: "End",
    page_down: "PageDown",
    page_up: "PageUp",
    // Actions
    paste_selected: "Enter",
    delete_selected: "Delete",
    pin_selected: "p",
    copy_selected: "c",
    search_focus: "/",
    close_window: "Escape",
    toggle_preview: "ArrowRight",
    close_preview: "ArrowLeft",
    context_menu: "Shift+F10",
    select_all: "Ctrl+a",
    clear_search: "Ctrl+l",
    // Tabs
    tab_clipboard: "1",
    tab_emojis: "2",
    tab_symbols: "3",
    tab_gifs: "4",
    tab_snippets: "5",
    // Cheat sheet
    show_cheatsheet: "Ctrl+/",
};

/** Vim-mode additional/override bindings (active only when vimMode is on). */
const VIM_BINDINGS = {
    select_next: "j",
    select_prev: "k",
    select_first: "gg",
    select_last: "G",
    paste_selected: "y",
    delete_selected: "dd",
    pin_selected: "p",
    search_focus: "/",
    toggle_preview: "o",
    show_cheatsheet: "?",
    page_down: "Ctrl+d",
    page_up: "Ctrl+u",
    vim_toggle_select: "v",
};

const KEYBINDING_STORAGE_KEY = "linvclip_keybindings";
const VIM_MODE_STORAGE_KEY = "linvclip_vim_mode";

const KeybindingContext = createContext(null);

/**
 * Parse a key event into a combo string like "Ctrl+Shift+k".
 */
function eventToCombo(e) {
    const parts = [];
    if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey && e.key.length > 1) parts.push("Shift"); // don't add Shift for printable chars
    // Normalize key
    let key = e.key;
    if (key === " ") key = "Space";
    parts.push(key);
    return parts.join("+");
}

/**
 * Check if a key event matches a binding combo string.
 */
function matchesCombo(e, combo) {
    if (!combo) return false;
    return eventToCombo(e) === combo;
}

export function KeybindingProvider({ children }) {
    const [bindings, setBindings] = useState(() => {
        try {
            const stored = localStorage.getItem(KEYBINDING_STORAGE_KEY);
            if (stored) return { ...DEFAULT_BINDINGS, ...JSON.parse(stored) };
        } catch {}
        return { ...DEFAULT_BINDINGS };
    });

    const [vimMode, setVimMode] = useState(() => {
        return localStorage.getItem(VIM_MODE_STORAGE_KEY) === "true";
    });

    // Vim mode state: "normal" or "insert" (insert = search bar focused)
    const [vimState, setVimState] = useState("normal");

    // For multi-key combos like "gg" and "dd"
    const lastKeyRef = useRef({ key: "", time: 0 });

    // Persist bindings
    useEffect(() => {
        localStorage.setItem(KEYBINDING_STORAGE_KEY, JSON.stringify(bindings));
    }, [bindings]);

    useEffect(() => {
        localStorage.setItem(VIM_MODE_STORAGE_KEY, String(vimMode));
    }, [vimMode]);

    // Track whether search is focused for vim mode
    const handleSearchFocus = useCallback(() => setVimState("insert"), []);
    const handleSearchBlur = useCallback(() => setVimState("normal"), []);

    /**
     * Get the effective binding for an action, considering vim mode.
     */
    const getBinding = useCallback(
        (action) => {
            if (vimMode && VIM_BINDINGS[action]) {
                return VIM_BINDINGS[action];
            }
            return bindings[action] || DEFAULT_BINDINGS[action];
        },
        [bindings, vimMode]
    );

    /**
     * Update a single keybinding.
     */
    const setBinding = useCallback((action, combo) => {
        setBindings((prev) => ({ ...prev, [action]: combo }));
    }, []);

    /**
     * Reset all bindings to defaults.
     */
    const resetBindings = useCallback(() => {
        setBindings({ ...DEFAULT_BINDINGS });
    }, []);

    /**
     * Check if a combo conflicts with an existing binding.
     * Returns the conflicting action name or null.
     */
    const findConflict = useCallback(
        (combo, excludeAction) => {
            for (const [action, bound] of Object.entries(bindings)) {
                if (action !== excludeAction && bound === combo) return action;
            }
            return null;
        },
        [bindings]
    );

    /**
     * Resolve a keyboard event to an action name, handling multi-key combos.
     * Returns { action, preventDefault } or null.
     */
    const resolveAction = useCallback(
        (e) => {
            const isInput = document.activeElement?.tagName === "INPUT" ||
                            document.activeElement?.tagName === "TEXTAREA";

            // In vim mode + insert state, only Escape works
            if (vimMode && vimState === "insert") {
                if (e.key === "Escape") return { action: "exit_search", preventDefault: true };
                return null;
            }

            // If focused on input (non-vim), let Ctrl shortcuts through, only block single keys
            if (isInput && !vimMode) {
                if (e.key === "Escape") return { action: "close_window", preventDefault: true };
                if ((e.ctrlKey || e.metaKey) && e.key === "a") return null; // let native select-all work in input
                if ((e.ctrlKey || e.metaKey) && (e.key === "+" || e.key === "=" || e.key === "-" || e.key === "0")) {
                    return null; // let zoom through
                }
                if (e.ctrlKey || e.metaKey) {
                    // Match against effective bindings for Ctrl combos (e.g., Ctrl+L, Ctrl+/)
                    for (const [action, bound] of Object.entries(bindings)) {
                        if (matchesCombo(e, bound)) return { action, preventDefault: true };
                    }
                }
                return null;
            }

            const now = Date.now();
            const combo = eventToCombo(e);

            // Check multi-key vim combos: gg, dd
            if (vimMode) {
                const last = lastKeyRef.current;
                if (e.key === "g" && last.key === "g" && now - last.time < 500) {
                    lastKeyRef.current = { key: "", time: 0 };
                    return { action: "select_first", preventDefault: true };
                }
                if (e.key === "d" && last.key === "d" && now - last.time < 500) {
                    lastKeyRef.current = { key: "", time: 0 };
                    return { action: "delete_selected", preventDefault: true };
                }
                lastKeyRef.current = { key: e.key, time: now };

                // Single "g" or "d" — wait for second press, don't act yet
                if (e.key === "g" || e.key === "d") return null;
            }

            // Match against all effective bindings
            const effectiveBindings = vimMode
                ? { ...bindings, ...VIM_BINDINGS }
                : bindings;

            for (const [action, bound] of Object.entries(effectiveBindings)) {
                if (matchesCombo(e, bound)) {
                    // Skip tab keys and single-char keys if input is focused
                    if (isInput && bound.length === 1) continue;
                    return { action, preventDefault: true };
                }
            }

            // Zoom shortcuts (always available)
            if (e.ctrlKey || e.metaKey) {
                if (e.key === "+" || e.key === "=") return { action: "zoom_in", preventDefault: true };
                if (e.key === "-") return { action: "zoom_out", preventDefault: true };
                if (e.key === "0") return { action: "zoom_reset", preventDefault: true };
            }

            return null;
        },
        [bindings, vimMode, vimState]
    );

    return (
        <KeybindingContext.Provider
            value={{
                bindings,
                vimMode,
                vimState,
                setVimMode,
                setVimState,
                getBinding,
                setBinding,
                resetBindings,
                findConflict,
                resolveAction,
                handleSearchFocus,
                handleSearchBlur,
                DEFAULT_BINDINGS,
                VIM_BINDINGS,
                eventToCombo,
            }}
        >
            {children}
        </KeybindingContext.Provider>
    );
}

export function useKeybindings() {
    const ctx = useContext(KeybindingContext);
    if (!ctx) throw new Error("useKeybindings must be used within <KeybindingProvider>");
    return ctx;
}

/** All bindable actions grouped by category. */
export const KEYBINDING_ACTIONS = {
    Navigation: [
        { action: "select_next", label: "keybind.select_next" },
        { action: "select_prev", label: "keybind.select_prev" },
        { action: "select_first", label: "keybind.select_first" },
        { action: "select_last", label: "keybind.select_last" },
        { action: "page_down", label: "keybind.page_down" },
        { action: "page_up", label: "keybind.page_up" },
    ],
    Actions: [
        { action: "paste_selected", label: "keybind.paste_selected" },
        { action: "delete_selected", label: "keybind.delete_selected" },
        { action: "pin_selected", label: "keybind.pin_selected" },
        { action: "copy_selected", label: "keybind.copy_selected" },
        { action: "search_focus", label: "keybind.search_focus" },
        { action: "close_window", label: "keybind.close_window" },
        { action: "toggle_preview", label: "keybind.toggle_preview" },
        { action: "context_menu", label: "keybind.context_menu" },
        { action: "select_all", label: "keybind.select_all" },
        { action: "clear_search", label: "keybind.clear_search" },
        { action: "show_cheatsheet", label: "keybind.show_cheatsheet" },
    ],
    Tabs: [
        { action: "tab_clipboard", label: "keybind.tab_clipboard" },
        { action: "tab_emojis", label: "keybind.tab_emojis" },
        { action: "tab_symbols", label: "keybind.tab_symbols" },
        { action: "tab_gifs", label: "keybind.tab_gifs" },
        { action: "tab_snippets", label: "keybind.tab_snippets" },
    ],
};
