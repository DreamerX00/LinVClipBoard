import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-shell";
import { useTranslation } from "../i18n/index.jsx";
import { useKeybindings, KEYBINDING_ACTIONS } from "../contexts/KeybindingContext.jsx";
import UpdateModal from "./UpdateModal.jsx";

const WINDOW_PRESETS = [
    { label: "Compact",   w: 380, h: 480 },
    { label: "Default",   w: 420, h: 520 },
    { label: "Medium",    w: 480, h: 580 },
    { label: "Large",     w: 540, h: 640 },
    { label: "Wide",      w: 600, h: 700 },
];

const SETTINGS_TABS = [
    { id: "general",    icon: "🌐", key: "settings.tab_general" },
    { id: "appearance", icon: "🎨", key: "settings.tab_appearance" },
    { id: "window",     icon: "🖥️", key: "settings.tab_window" },
    { id: "keyboard",   icon: "⌨️", key: "settings.tab_keyboard" },
    { id: "features",   icon: "🧠", key: "settings.tab_features" },
    { id: "advanced",   icon: "⚙️", key: "settings.tab_advanced" },
];

function SettingsPanel({ onClose, zoom, onZoomChange }) {
    const { t, lang, setLang, availableLanguages } = useTranslation();
    const { bindings, vimMode, setVimMode, setBinding, resetBindings, findConflict, eventToCombo } = useKeybindings();
    const [editingAction, setEditingAction] = useState(null);
    const [keybindConflict, setKeybindConflict] = useState(null);
    const [config, setConfig] = useState(null);
    const [error, setError] = useState(null);
    const configRef = useRef(null);
    const dirtyRef = useRef(false);
    const [updateStatus, setUpdateStatus] = useState(null);
    const [showUpdateModal, setShowUpdateModal] = useState(false);
    const [appVersion, setAppVersion] = useState("");
    const [showUpdatePopup, setShowUpdatePopup] = useState(null);
    const [accentColor, setAccentColor] = useState(() => localStorage.getItem("accent_color") || "auto");
    const [activeTab, setActiveTab] = useState("general");

    // Window size state
    const [winSizePreset, setWinSizePreset] = useState(() => localStorage.getItem("winSizePreset") || "Default");
    const [customW, setCustomW] = useState(() => parseInt(localStorage.getItem("windowWidth")) || 420);
    const [customH, setCustomH] = useState(() => parseInt(localStorage.getItem("windowHeight")) || 520);
    const [previewWidth, setPreviewWidth] = useState(() => parseInt(localStorage.getItem("previewWidth")) || 380);

    useEffect(() => {
        loadConfig();
        invoke("get_app_version").then(setAppVersion).catch(() => {});
    }, []);

    const loadConfig = async () => {
        try {
            const cfg = await invoke("get_config");
            setConfig(cfg);
            configRef.current = cfg;
            if (cfg.ui?.language && cfg.ui.language !== lang) {
                setLang(cfg.ui.language);
            }
        } catch (err) {
            setError(String(err));
        }
    };

    const autoSave = useCallback(async () => {
        if (!dirtyRef.current || !configRef.current) return;
        try {
            await invoke("save_config", { config: configRef.current });
        } catch (err) {
            console.error("Auto-save failed:", err);
        }
    }, []);

    const handleClose = useCallback(() => {
        autoSave().then(() => onClose());
    }, [autoSave, onClose]);

    const [applyToast, setApplyToast] = useState(false);
    const handleApply = useCallback(async () => {
        if (configRef.current) {
            try {
                await invoke("save_config", { config: configRef.current });
                dirtyRef.current = false;
            } catch (err) {
                console.error("Apply save failed:", err);
            }
        }

        const cfg = configRef.current;
        if (cfg?.ui?.theme) {
            const resolved = cfg.ui.theme === "auto"
                ? (window.matchMedia?.("(prefers-color-scheme: light)")?.matches ? "catppuccin-latte" : "catppuccin-mocha")
                : cfg.ui.theme;
            document.documentElement.setAttribute("data-theme", resolved);
            localStorage.setItem("theme", cfg.ui.theme);
        }

        const accent = accentColor;
        if (accent !== "auto") {
            document.documentElement.style.setProperty("--accent", accent);
            document.documentElement.style.setProperty("--accent-hover", accent + "cc");
            document.documentElement.style.setProperty("--accent-active", accent);
            document.documentElement.style.setProperty("--accent-glow", accent + "4d");
            document.documentElement.style.setProperty("--border-focus", accent + "80");
        }

        if (onZoomChange && cfg?.ui?.zoom) {
            onZoomChange(cfg.ui.zoom);
        }

        try {
            const { LogicalSize } = await import("@tauri-apps/api/dpi");
            const win = getCurrentWindow();
            const w = parseInt(localStorage.getItem("windowWidth")) || 420;
            const h = parseInt(localStorage.getItem("windowHeight")) || 520;
            const pw = parseInt(localStorage.getItem("previewWidth")) || 380;
            const showingPreview = localStorage.getItem("showPreview") === "true";
            const totalW = showingPreview ? w + pw : w;
            document.documentElement.style.setProperty("--list-width", w + "px");
            await win.setSize(new LogicalSize(totalW, h));
        } catch (err) {
            console.error("Apply resize failed:", err);
        }

        setApplyToast(true);
        setTimeout(() => setApplyToast(false), 2000);
    }, [onZoomChange]);

    const updateConfig = (updater) => {
        dirtyRef.current = true;
        setConfig((prev) => {
            const next = updater(prev);
            configRef.current = next;
            return next;
        });
    };

    const handleLanguageChange = (newLang) => {
        setLang(newLang);
        updateConfig((prev) => ({
            ...prev,
            ui: { ...prev.ui, language: newLang },
        }));
    };

    const handlePositionChange = (mode) => {
        updateConfig((prev) => ({
            ...prev,
            ui: { ...prev.ui, window_position: mode },
        }));
    };

    const handleZoomChange = (val) => {
        const z = Math.max(50, Math.min(200, parseInt(val) || 100));
        if (onZoomChange) onZoomChange(z);
        updateConfig((prev) => ({
            ...prev,
            ui: { ...prev.ui, zoom: z },
        }));
    };

    const handleThemeChange = (newTheme) => {
        updateConfig((prev) => ({
            ...prev,
            ui: { ...prev.ui, theme: newTheme },
        }));
        const resolved = newTheme === "auto"
            ? (window.matchMedia?.("(prefers-color-scheme: light)")?.matches ? "catppuccin-latte" : "catppuccin-mocha")
            : newTheme;
        document.documentElement.setAttribute("data-theme", resolved);
        localStorage.setItem("theme", newTheme);
    };

    const handleAccentChange = (color) => {
        setAccentColor(color);
        localStorage.setItem("accent_color", color);
        updateConfig((prev) => ({
            ...prev,
            ui: { ...prev.ui, accent_color: color },
        }));
        if (color !== "auto") {
            document.documentElement.style.setProperty("--accent", color);
            document.documentElement.style.setProperty("--accent-hover", color + "cc");
            document.documentElement.style.setProperty("--accent-active", color);
            document.documentElement.style.setProperty("--accent-glow", color + "4d");
            document.documentElement.style.setProperty("--border-focus", color + "80");
        } else {
            document.documentElement.style.removeProperty("--accent");
            document.documentElement.style.removeProperty("--accent-hover");
            document.documentElement.style.removeProperty("--accent-active");
            document.documentElement.style.removeProperty("--accent-glow");
            document.documentElement.style.removeProperty("--border-focus");
        }
    };

    const applyWindowSize = useCallback(async (w, h) => {
        w = Math.max(320, Math.min(1200, w));
        h = Math.max(400, Math.min(900, h));
        localStorage.setItem("windowWidth", String(w));
        localStorage.setItem("windowHeight", String(h));
        setCustomW(w);
        setCustomH(h);
        try {
            const { LogicalSize } = await import("@tauri-apps/api/dpi");
            const win = getCurrentWindow();
            const showingPreview = localStorage.getItem("showPreview") === "true";
            const pw = parseInt(localStorage.getItem("previewWidth")) || 380;
            const totalW = showingPreview ? w + pw : w;
            document.documentElement.style.setProperty("--list-width", w + "px");
            await win.setSize(new LogicalSize(totalW, h));
        } catch {}
    }, []);

    const handlePresetChange = useCallback((label) => {
        setWinSizePreset(label);
        localStorage.setItem("winSizePreset", label);
        if (label === "Custom") return;
        const preset = WINDOW_PRESETS.find((p) => p.label === label);
        if (preset) applyWindowSize(preset.w, preset.h);
    }, [applyWindowSize]);

    const handlePreviewWidthChange = useCallback(async (val) => {
        const pw = Math.max(250, Math.min(600, parseInt(val) || 380));
        setPreviewWidth(pw);
        localStorage.setItem("previewWidth", String(pw));
        try {
            const { LogicalSize } = await import("@tauri-apps/api/dpi");
            const win = getCurrentWindow();
            const showingPreview = localStorage.getItem("showPreview") === "true";
            const factor = await win.scaleFactor();
            const size = await win.innerSize();
            const logicalH = size.height / factor;
            const baseW = parseInt(localStorage.getItem("windowWidth")) || 420;
            if (showingPreview) {
                await win.setSize(new LogicalSize(baseW + pw, logicalH));
            }
        } catch {}
    }, []);

    const handleCheckUpdate = useCallback(async () => {
        setUpdateStatus("checking");
        setShowUpdatePopup(null);
        try {
            const info = await invoke("check_for_updates");
            setUpdateStatus(info);
            if (info.has_update) {
                setShowUpdateModal(true);
            } else {
                setShowUpdatePopup("up_to_date");
                setTimeout(() => setShowUpdatePopup(null), 4000);
            }
        } catch (err) {
            setUpdateStatus({ error: String(err) });
            setShowUpdatePopup("error");
            setTimeout(() => setShowUpdatePopup(null), 5000);
        }
    }, []);

    const handleOpenRelease = useCallback(async (url) => {
        try {
            await open(url);
        } catch (_) {
            window.open(url, "_blank");
        }
    }, []);

    const handleTrayItemsChange = (val) => {
        const v = Math.max(3, Math.min(15, parseInt(val) || 5));
        updateConfig((prev) => ({
            ...prev,
            ui: { ...prev.ui, tray_items: v },
        }));
    };

    if (!config) {
        return (
            <div className="settings-overlay" onClick={handleClose}>
                <div className="settings-panel" role="dialog" aria-label={t("settings.title")} aria-modal="true" onClick={(e) => e.stopPropagation()}>
                    <div className="settings-loading">{error || "Loading…"}</div>
                </div>
            </div>
        );
    }

    const currentZoom = zoom ?? config.ui?.zoom ?? 100;
    const currentPosition = config.ui?.window_position ?? "mouse";
    const currentTrayItems = config.ui?.tray_items ?? 5;

    /* ── Tab Content Renderers ── */

    const renderGeneralTab = () => (
        <>
            {/* Language */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.language")}</div>
                <select
                    className="language-select"
                    value={lang}
                    onChange={(e) => handleLanguageChange(e.target.value)}
                    aria-label={t("settings.language")}
                >
                    {availableLanguages.map((code) => (
                        <option key={code} value={code}>
                            {{ en: "English", pt: "Português", ja: "日本語", hi: "हिन्दी" }[code] || code}
                        </option>
                    ))}
                </select>
            </div>

            {/* Tray Menu Items */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.tray_items")}</div>
                <p className="settings-hint">{t("settings.tray_items_desc")}</p>
                <div className="zoom-slider-row">
                    <input
                        type="range"
                        min="3"
                        max="15"
                        step="1"
                        value={currentTrayItems}
                        onChange={(e) => handleTrayItemsChange(e.target.value)}
                        className="zoom-slider"
                        aria-label={t("settings.tray_items")}
                    />
                    <span className="zoom-value">{currentTrayItems}</span>
                </div>
            </div>
        </>
    );

    const renderAppearanceTab = () => (
        <>
            {/* Theme */}
            <div className="settings-section theme-grid-section">
                <div className="settings-section-label">{t("settings.theme")}</div>

                <div className="theme-grid-group-label">{t("settings.theme_system")}</div>
                <div className="theme-grid" role="radiogroup" aria-label={t("settings.theme")}>
                    {[
                        { value: "auto",  label: t("settings.theme_auto"),  colors: ["#818cf8","#f87171","#fbbf24","#34d399"] },
                        { value: "dark",  label: t("settings.theme_dark"),  colors: ["#0f0f19","#191928","#818cf8","#f1f5f9"] },
                        { value: "light", label: t("settings.theme_light"), colors: ["#f8fafc","#e2e8f0","#6366f1","#1e293b"] },
                    ].map((th) => (
                        <button
                            key={th.value}
                            type="button"
                            role="radio"
                            aria-checked={(config.ui?.theme ?? "dark") === th.value}
                            className={`theme-card${(config.ui?.theme ?? "dark") === th.value ? " theme-card-active" : ""}`}
                            onClick={() => handleThemeChange(th.value)}
                        >
                            <div className="theme-card-swatches">
                                {th.colors.map((c, i) => <span key={i} style={{ background: c }} />)}
                            </div>
                            <div className="theme-card-name">{th.label}</div>
                        </button>
                    ))}
                </div>

                <div className="theme-grid-group-label">{t("settings.theme_catppuccin")}</div>
                <div className="theme-grid">
                    {[
                        { value: "catppuccin-latte",     label: "Latte",     colors: ["#eff1f5","#dce0e8","#1e66f5","#4c4f69"] },
                        { value: "catppuccin-frappe",    label: "Frappé",    colors: ["#303446","#3b3f4d","#8caaee","#c6d0f5"] },
                        { value: "catppuccin-macchiato", label: "Macchiato", colors: ["#24273a","#2c2f43","#8aadf4","#cad3f5"] },
                        { value: "catppuccin-mocha",     label: "Mocha",     colors: ["#1e1e2e","#27273a","#89b4fa","#cdd6f4"] },
                    ].map((th) => (
                        <button
                            key={th.value}
                            type="button"
                            role="radio"
                            aria-checked={(config.ui?.theme ?? "dark") === th.value}
                            className={`theme-card${(config.ui?.theme ?? "dark") === th.value ? " theme-card-active" : ""}`}
                            onClick={() => handleThemeChange(th.value)}
                        >
                            <div className="theme-card-swatches">
                                {th.colors.map((c, i) => <span key={i} style={{ background: c }} />)}
                            </div>
                            <div className="theme-card-name">{th.label}</div>
                        </button>
                    ))}
                </div>

                <div className="theme-grid-group-label">{t("settings.theme_classic")}</div>
                <div className="theme-grid">
                    {[
                        { value: "nord",    label: "Nord",    colors: ["#2e3440","#3b4252","#88c0d0","#eceff4"] },
                        { value: "dracula", label: "Dracula", colors: ["#282a36","#44475a","#bd93f9","#f8f8f2"] },
                    ].map((th) => (
                        <button
                            key={th.value}
                            type="button"
                            role="radio"
                            aria-checked={(config.ui?.theme ?? "dark") === th.value}
                            className={`theme-card${(config.ui?.theme ?? "dark") === th.value ? " theme-card-active" : ""}`}
                            onClick={() => handleThemeChange(th.value)}
                        >
                            <div className="theme-card-swatches">
                                {th.colors.map((c, i) => <span key={i} style={{ background: c }} />)}
                            </div>
                            <div className="theme-card-name">{th.label}</div>
                        </button>
                    ))}
                </div>
            </div>

            {/* Accent Color */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.accent_color")}</div>
                <div className="accent-picker">
                    <div className="accent-presets" role="radiogroup" aria-label={t("settings.accent_color")}>
                        <button
                            type="button"
                            role="radio"
                            aria-checked={accentColor === "auto"}
                            className={`accent-dot accent-dot-auto${accentColor === "auto" ? " accent-dot-active" : ""}`}
                            onClick={() => handleAccentChange("auto")}
                            title={t("settings.accent_auto")}
                        />
                        {[
                            { color: "#818cf8", name: "Indigo" },
                            { color: "#8b5cf6", name: "Purple" },
                            { color: "#34d399", name: "Green" },
                            { color: "#f472b6", name: "Pink" },
                            { color: "#fb923c", name: "Orange" },
                            { color: "#2dd4bf", name: "Teal" },
                            { color: "#f87171", name: "Red" },
                            { color: "#fbbf24", name: "Amber" },
                            { color: "#60a5fa", name: "Blue" },
                        ].map((p) => (
                            <button
                                key={p.color}
                                type="button"
                                role="radio"
                                aria-checked={accentColor === p.color}
                                className={`accent-dot${accentColor === p.color ? " accent-dot-active" : ""}`}
                                style={{ background: p.color }}
                                onClick={() => handleAccentChange(p.color)}
                                title={p.name}
                            />
                        ))}
                    </div>
                    <div className="accent-custom-row">
                        <input
                            type="color"
                            value={accentColor !== "auto" ? accentColor : "#818cf8"}
                            onChange={(e) => handleAccentChange(e.target.value)}
                            aria-label="Custom accent color"
                        />
                        <span className="accent-custom-label">{t("settings.accent_custom")}</span>
                    </div>
                </div>
            </div>
        </>
    );

    const renderWindowTab = () => (
        <>
            {/* Window Position */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.window_position")}</div>
                <div className="toggle-group" role="radiogroup" aria-label={t("settings.window_position")}>
                    <button
                        type="button"
                        role="radio"
                        aria-checked={currentPosition === "fixed"}
                        className={`toggle-btn${currentPosition === "fixed" ? " toggle-btn-active" : ""}`}
                        onClick={() => handlePositionChange("fixed")}
                    >
                        {t("settings.position_fixed")}
                    </button>
                    <button
                        type="button"
                        role="radio"
                        aria-checked={currentPosition === "mouse"}
                        className={`toggle-btn${currentPosition === "mouse" ? " toggle-btn-active" : ""}`}
                        onClick={() => handlePositionChange("mouse")}
                    >
                        {t("settings.position_mouse")}
                    </button>
                </div>
            </div>

            {/* Zoom */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.size_zoom")}</div>
                <div className="zoom-slider-row">
                    <input
                        type="range"
                        min="50"
                        max="200"
                        step="10"
                        value={currentZoom}
                        onChange={(e) => handleZoomChange(e.target.value)}
                        className="zoom-slider"
                        aria-label={t("settings.size_zoom")}
                    />
                    <span className="zoom-value">{currentZoom}%</span>
                </div>
            </div>

            {/* Window Size */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.window_size")}</div>
                <div className="win-size-presets">
                    <select
                        className="win-size-select"
                        value={winSizePreset}
                        onChange={(e) => handlePresetChange(e.target.value)}
                        aria-label={t("settings.window_size")}
                    >
                        {WINDOW_PRESETS.map((p) => (
                            <option key={p.label} value={p.label}>
                                {p.label} ({p.w}×{p.h})
                            </option>
                        ))}
                        <option value="Custom">{t("settings.win_custom")}</option>
                    </select>
                </div>
                {winSizePreset === "Custom" && (
                    <div className="win-size-custom">
                        <div className="win-size-field">
                            <label>{t("settings.win_width")}</label>
                            <input
                                type="number"
                                min="320"
                                max="1200"
                                value={customW}
                                onChange={(e) => setCustomW(parseInt(e.target.value) || 420)}
                                onBlur={() => applyWindowSize(customW, customH)}
                                onKeyDown={(e) => e.key === "Enter" && applyWindowSize(customW, customH)}
                            />
                            <span className="win-size-unit">px</span>
                        </div>
                        <div className="win-size-field">
                            <label>{t("settings.win_height")}</label>
                            <input
                                type="number"
                                min="400"
                                max="900"
                                value={customH}
                                onChange={(e) => setCustomH(parseInt(e.target.value) || 520)}
                                onBlur={() => applyWindowSize(customW, customH)}
                                onKeyDown={(e) => e.key === "Enter" && applyWindowSize(customW, customH)}
                            />
                            <span className="win-size-unit">px</span>
                        </div>
                    </div>
                )}
                <div className="win-size-preview-row">
                    <label>{t("settings.preview_panel_width")}</label>
                    <div className="win-size-field">
                        <input
                            type="range"
                            min="250"
                            max="600"
                            step="10"
                            value={previewWidth}
                            onChange={(e) => handlePreviewWidthChange(e.target.value)}
                            className="zoom-slider"
                        />
                        <span className="zoom-value">{previewWidth}px</span>
                    </div>
                </div>
            </div>
        </>
    );

    const renderKeyboardTab = () => (
        <>
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.keyboard_shortcuts")}</div>

                {/* Vim mode toggle */}
                <label className="checkbox-label">
                    <input
                        type="checkbox"
                        checked={vimMode}
                        onChange={(e) => setVimMode(e.target.checked)}
                    />
                    {t("settings.vim_mode")}
                </label>
                {vimMode && (
                    <p className="settings-hint">{t("settings.vim_mode_desc")}</p>
                )}

                {/* Keybinding editor */}
                <div className="keybind-editor">
                    {Object.entries(KEYBINDING_ACTIONS).map(([category, actions]) => (
                        <div key={category} className="keybind-category">
                            <div className="keybind-category-title">{category}</div>
                            {actions.map(({ action, label }) => (
                                <div key={action} className="keybind-row">
                                    <span className="keybind-action">{t(label)}</span>
                                    {editingAction === action ? (
                                        <button
                                            className="keybind-key keybind-key-editing"
                                            autoFocus
                                            onKeyDown={(e) => {
                                                e.preventDefault();
                                                e.stopPropagation();
                                                if (e.key === "Escape") {
                                                    setEditingAction(null);
                                                    setKeybindConflict(null);
                                                    return;
                                                }
                                                const combo = eventToCombo(e);
                                                const conflict = findConflict(combo, action);
                                                if (conflict) {
                                                    setKeybindConflict({ combo, action: conflict });
                                                } else {
                                                    setBinding(action, combo);
                                                    setEditingAction(null);
                                                    setKeybindConflict(null);
                                                }
                                            }}
                                            onBlur={() => { setEditingAction(null); setKeybindConflict(null); }}
                                        >
                                            {t("settings.press_key")}
                                        </button>
                                    ) : (
                                        <button
                                            className="keybind-key"
                                            onClick={() => { setEditingAction(action); setKeybindConflict(null); }}
                                        >
                                            {bindings[action] || "—"}
                                        </button>
                                    )}
                                </div>
                            ))}
                        </div>
                    ))}
                </div>

                {keybindConflict && (
                    <div className="keybind-conflict" role="alert">
                        ⚠️ {t("settings.conflict_warning")}: "{keybindConflict.combo}" → {keybindConflict.action}
                    </div>
                )}

                <button className="keybind-reset-btn" onClick={resetBindings}>
                    🔄 {t("settings.reset_defaults")}
                </button>

                <div className="shortcuts-info">
                    <p className="shortcuts-instruction">{t("settings.shortcut_instructions")}</p>
                    <ol className="shortcuts-steps">
                        <li>{t("settings.shortcut_step1")}</li>
                        <li>{t("settings.shortcut_step2")}</li>
                        <li>{t("settings.shortcut_step3")}</li>
                    </ol>
                </div>
            </div>
        </>
    );

    const renderFeaturesTab = () => (
        <>
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.smart_features")}</div>
                <label className="checkbox-label">
                    <input
                        type="checkbox"
                        checked={config.features?.auto_ocr ?? false}
                        onChange={(e) =>
                            updateConfig((prev) => ({
                                ...prev,
                                features: { ...prev.features, auto_ocr: e.target.checked },
                            }))
                        }
                    />
                    {t("settings.auto_ocr")}
                </label>
                <label className="checkbox-label">
                    <input
                        type="checkbox"
                        checked={config.features?.smart_paste ?? true}
                        onChange={(e) =>
                            updateConfig((prev) => ({
                                ...prev,
                                features: { ...prev.features, smart_paste: e.target.checked },
                            }))
                        }
                    />
                    {t("settings.smart_paste")}
                </label>
                <label className="checkbox-label">
                    <input
                        type="checkbox"
                        checked={config.features?.redact_sensitive ?? true}
                        onChange={(e) =>
                            updateConfig((prev) => ({
                                ...prev,
                                features: { ...prev.features, redact_sensitive: e.target.checked },
                            }))
                        }
                    />
                    {t("settings.redact_sensitive")}
                </label>
                <label htmlFor="sensitive-expiry">{t("settings.sensitive_expiry")}</label>
                <input
                    id="sensitive-expiry"
                    type="number"
                    min="0"
                    max="1440"
                    value={config.security?.sensitive_expiry_minutes ?? 0}
                    onChange={(e) =>
                        updateConfig((prev) => ({
                            ...prev,
                            security: { ...prev.security, sensitive_expiry_minutes: parseInt(e.target.value) || 0 },
                        }))
                    }
                />
                <label className="checkbox-label">
                    <input
                        type="checkbox"
                        checked={config.security?.clear_after_paste ?? false}
                        onChange={(e) =>
                            updateConfig((prev) => ({
                                ...prev,
                                security: { ...prev.security, clear_after_paste: e.target.checked },
                            }))
                        }
                    />
                    {t("settings.clear_after_paste")}
                </label>
            </div>
        </>
    );

    const renderAdvancedTab = () => (
        <>
            {/* Daemon */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.daemon")}</div>
                <label htmlFor="poll-interval">{t("settings.poll_interval")}</label>
                <input
                    id="poll-interval"
                    type="number"
                    min="50"
                    max="5000"
                    step="50"
                    value={config.daemon?.poll_interval_ms ?? 250}
                    onChange={(e) =>
                        updateConfig((prev) => ({
                            ...prev,
                            daemon: { ...prev.daemon, poll_interval_ms: parseInt(e.target.value) || 250 },
                        }))
                    }
                />
                <label htmlFor="log-level">{t("settings.log_level")}</label>
                <select
                    id="log-level"
                    value={config.daemon?.log_level ?? "info"}
                    onChange={(e) =>
                        updateConfig((prev) => ({
                            ...prev,
                            daemon: { ...prev.daemon, log_level: e.target.value },
                        }))
                    }
                >
                    <option value="trace">Trace</option>
                    <option value="debug">Debug</option>
                    <option value="info">Info</option>
                    <option value="warn">Warn</option>
                    <option value="error">Error</option>
                </select>
            </div>

            {/* Storage */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.storage")}</div>
                <label htmlFor="max-items">{t("settings.max_items")}</label>
                <input
                    id="max-items"
                    type="number"
                    min="100"
                    max="100000"
                    value={config.storage?.max_items ?? 10000}
                    onChange={(e) =>
                        updateConfig((prev) => ({
                            ...prev,
                            storage: { ...prev.storage, max_items: parseInt(e.target.value) || 10000 },
                        }))
                    }
                />
                <label htmlFor="expiry-days">{t("settings.expiry_days")}</label>
                <input
                    id="expiry-days"
                    type="number"
                    min="1"
                    max="365"
                    value={config.storage?.expiry_days ?? 30}
                    onChange={(e) =>
                        updateConfig((prev) => ({
                            ...prev,
                            storage: { ...prev.storage, expiry_days: parseInt(e.target.value) || 30 },
                        }))
                    }
                />
                <label htmlFor="max-size">{t("settings.max_size")}</label>
                <input
                    id="max-size"
                    type="number"
                    min="1"
                    max="200"
                    value={Math.round((config.storage?.max_item_size_bytes ?? 52428800) / 1048576)}
                    onChange={(e) =>
                        updateConfig((prev) => ({
                            ...prev,
                            storage: {
                                ...prev.storage,
                                max_item_size_bytes: (parseInt(e.target.value) || 50) * 1048576,
                            },
                        }))
                    }
                />
            </div>

            {/* Security */}
            <div className="settings-section">
                <div className="settings-section-label">{t("settings.security")}</div>
                <label className="checkbox-label">
                    <input
                        type="checkbox"
                        checked={config.security?.incognito ?? false}
                        onChange={(e) =>
                            updateConfig((prev) => ({
                                ...prev,
                                security: { ...prev.security, incognito: e.target.checked },
                            }))
                        }
                    />
                    {t("settings.incognito")}
                </label>
                <label htmlFor="blacklist">{t("settings.blacklisted_apps")}</label>
                <input
                    id="blacklist"
                    type="text"
                    value={(config.security?.blacklisted_apps ?? []).join(", ")}
                    onChange={(e) =>
                        updateConfig((prev) => ({
                            ...prev,
                            security: {
                                ...prev.security,
                                blacklisted_apps: e.target.value
                                    .split(",")
                                    .map((s) => s.trim())
                                    .filter(Boolean),
                            },
                        }))
                    }
                />
            </div>

            {/* Updates */}
            <div className="settings-section settings-update-section">
                <div className="settings-section-label">{t("settings.updates")}</div>
                <div className="settings-update-row">
                    <span className="settings-version">v{appVersion || "…"}</span>
                    <button
                        className="settings-update-btn"
                        onClick={handleCheckUpdate}
                        disabled={updateStatus === "checking"}
                    >
                        {updateStatus === "checking" ? t("settings.checking") : t("settings.check_updates")}
                    </button>
                </div>
            </div>

            {/* "Up to date" / error popup */}
            {showUpdatePopup && (
                <div className="update-popup-overlay" onClick={() => setShowUpdatePopup(null)}>
                    <div className="update-popup" onClick={(e) => e.stopPropagation()}>
                        {showUpdatePopup === "up_to_date" ? (
                            <>
                                <span className="update-popup-icon">✅</span>
                                <h3 className="update-popup-title">{t("settings.up_to_date")}</h3>
                                <p className="update-popup-desc">LinVClipBoard v{appVersion}</p>
                            </>
                        ) : (
                            <>
                                <span className="update-popup-icon">⚠️</span>
                                <h3 className="update-popup-title">{updateStatus?.error || "Error"}</h3>
                            </>
                        )}
                    </div>
                </div>
            )}

            {/* Update modal */}
            {showUpdateModal && updateStatus?.has_update && (
                <UpdateModal
                    updateInfo={updateStatus}
                    onClose={() => setShowUpdateModal(false)}
                />
            )}
        </>
    );

    const TAB_RENDERERS = {
        general: renderGeneralTab,
        appearance: renderAppearanceTab,
        window: renderWindowTab,
        keyboard: renderKeyboardTab,
        features: renderFeaturesTab,
        advanced: renderAdvancedTab,
    };

    return (
        <div className="settings-overlay" onClick={handleClose}>
            <div className="settings-panel settings-panel-tabbed" role="dialog" aria-label={t("settings.title")} aria-modal="true" onClick={(e) => e.stopPropagation()}>
                <div className="settings-header">
                    <h2 id="settings-title">{t("settings.title")}</h2>
                    <button className="settings-close" onClick={handleClose} aria-label="Close">✕</button>
                </div>

                {/* Tab bar */}
                <div className="settings-tab-bar" role="tablist">
                    {SETTINGS_TABS.map((tab) => (
                        <button
                            key={tab.id}
                            role="tab"
                            aria-selected={activeTab === tab.id}
                            className={`settings-tab${activeTab === tab.id ? " settings-tab-active" : ""}`}
                            onClick={() => setActiveTab(tab.id)}
                        >
                            <span className="settings-tab-icon">{tab.icon}</span>
                            <span className="settings-tab-label">{t(tab.key)}</span>
                        </button>
                    ))}
                </div>

                {/* Tab content */}
                <div className="settings-body" role="form" aria-labelledby="settings-title">
                    {error && <div className="settings-error" role="alert">{error}</div>}
                    {TAB_RENDERERS[activeTab]?.()}
                </div>

                {/* Sticky Apply footer */}
                <div className="settings-footer">
                    <div className="settings-apply-row">
                        <button className="settings-apply-btn" onClick={handleApply}>
                            ✅ {t("settings.apply")}
                        </button>
                    </div>
                    {applyToast && <div className="settings-apply-toast">✓ {t("settings.applied")}</div>}
                    <div className="settings-save-hint-footer">{t("settings.close_to_save")}</div>
                </div>
            </div>
        </div>
    );
}

export default SettingsPanel;
