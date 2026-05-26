import { useRef, useEffect, useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/index.jsx";
import { detectSmartContent, hasSensitiveContent, redactText } from "../utils/smartDetect.js";

/** Small hook to lazily load image thumbnails (#38). */
function useImagePreview(item) {
    const [src, setSrc] = useState(null);

    useEffect(() => {
        if (item.content_type !== "image") { setSrc(null); return; }
        let cancelled = false;
        invoke("get_image_base64", { path: item.content })
            .then((data) => { if (!cancelled) setSrc(data); })
            .catch(() => { if (!cancelled) setSrc(null); });
        return () => { cancelled = true; };
    }, [item.id, item.content, item.content_type]);

    return src;
}

function ClipboardList({
    items,
    selectedIndex,
    onPaste,
    onPin,
    onDelete,
    onLoadMore,
    loading,
    hasMore,
    onContextMenu,
    selectedIds,
    onSelectToggle,
    onSelectRange,
    config,
    onToast,
    onItemUpdate,
}) {
    const { t } = useTranslation();
    const listRef = useRef(null);
    const selectedRef = useRef(null);

    // Scroll selected item into view
    useEffect(() => {
        if (selectedRef.current) {
            selectedRef.current.scrollIntoView({
                block: "nearest",
                behavior: "smooth",
            });
        }
    }, [selectedIndex]);

    // Infinite scroll
    const handleScroll = () => {
        if (!listRef.current || loading || !hasMore) return;
        const { scrollTop, scrollHeight, clientHeight } = listRef.current;
        if (scrollHeight - scrollTop - clientHeight < 100) {
            onLoadMore();
        }
    };

    const formatTime = (dateStr) => {
        if (!dateStr) return "--:--:--";
        const date = new Date(dateStr);
        if (isNaN(date.getTime())) return "--:--:--";
        const hh = String(date.getHours()).padStart(2, "0");
        const mm = String(date.getMinutes()).padStart(2, "0");
        const ss = String(date.getSeconds()).padStart(2, "0");
        return `${hh}:${mm}:${ss}`;
    };

    const getTypeIcon = (contentType) => {
        switch (contentType) {
            case "plain_text":
                return "📝";
            case "html":
                return "🌐";
            case "image":
                return "🖼️";
            case "rich_text":
                return "📄";
            case "files":
                return "📁";
            case "uri":
                return "🔗";
            default:
                return "📋";
        }
    };

    const formatPreview = (text, maxLen = 80) => {
        if (!text) return "";
        const clean = text.replace(/\n/g, " ↵ ").replace(/\s+/g, " ").trim();
        if (clean.length > maxLen) return clean.slice(0, maxLen - 3) + "...";
        return clean;
    };

    /** Heuristic: content looks like code if it has braces, semicolons, arrows, etc. */
    const looksLikeCode = (text) => {
        if (!text) return false;
        const codePatterns = /[{};=>]|function |const |let |var |import |def |class |<\/?\w+>/;
        return codePatterns.test(text);
    };

    if (items.length === 0 && !loading) {
        return (
            <div className="empty-state" role="status" aria-label={t("clipboard.empty_title")}>
                <span className="empty-icon">📋</span>
                <p className="empty-title">{t("clipboard.empty_title")}</p>
                <p className="empty-sub">{t("clipboard.empty_sub")}</p>
            </div>
        );
    }

    const isMultiSelect = selectedIds && selectedIds.size > 0;

    return (
        <div
            className="clipboard-list"
            ref={listRef}
            onScroll={handleScroll}
            role="listbox"
            aria-label="Clipboard items"
        >
            {items.map((item, index) => (
                <ClipItem
                    key={item.id}
                    item={item}
                    index={index}
                    selectedIndex={selectedIndex}
                    selectedRef={selectedRef}
                    onPaste={onPaste}
                    onPin={onPin}
                    onDelete={onDelete}
                    onContextMenu={onContextMenu}
                    getTypeIcon={getTypeIcon}
                    formatPreview={formatPreview}
                    formatTime={formatTime}
                    looksLikeCode={looksLikeCode}
                    t={t}
                    isChecked={selectedIds ? selectedIds.has(item.id) : false}
                    isMultiSelect={isMultiSelect}
                    onSelectToggle={onSelectToggle}
                    onSelectRange={onSelectRange}
                    config={config}
                    onToast={onToast}
                    onItemUpdate={onItemUpdate}
                />
            ))}

            {loading && (
                <div className="loading-indicator" role="status" aria-label="Loading more items">
                    <div className="spinner"></div>
                </div>
            )}
        </div>
    );
}

/** Format file list preview from JSON content. */
function formatFilesPreview(item) {
    try {
        const files = JSON.parse(item.content);
        if (!Array.isArray(files)) return item.preview_text;
        const names = files.map((f) => {
            const parts = f.split("/");
            return parts[parts.length - 1] || f;
        });
        if (names.length <= 3) return names.join(", ");
        return `${names.slice(0, 3).join(", ")} +${names.length - 3} more`;
    } catch {
        return item.preview_text;
    }
}

/** Extract domain from URI content. */
function extractDomain(content) {
    try {
        const stripped = content.replace(/^https?:\/\//, "");
        return stripped.split("/")[0] || content;
    } catch {
        return content;
    }
}

/** Type badge for rich content. */
function TypeBadge({ type: contentType }) {
    const labels = {
        html: "HTML",
        uri: "Link",
        files: "Files",
    };
    const label = labels[contentType];
    if (!label) return null;
    return <span className={`type-badge type-badge-${contentType}`}>{label}</span>;
}

/** Individual clip item – extracted for the image preview hook. */
function ClipItem({
    item,
    index,
    selectedIndex,
    selectedRef,
    onPaste,
    onPin,
    onDelete,
    onContextMenu,
    getTypeIcon,
    formatPreview,
    formatTime,
    looksLikeCode,
    t,
    isChecked,
    isMultiSelect,
    onSelectToggle,
    onSelectRange,
    config,
    onToast,
    onItemUpdate,
}) {
    const imgSrc = useImagePreview(item);
    const [ocrLoading, setOcrLoading] = useState(false);
    const tags = (() => {
        try {
            return JSON.parse(item.tags || "[]");
        } catch {
            return [];
        }
    })();

    const isCode = looksLikeCode(item.preview_text);

    // Smart paste detection
    const smartChips = useMemo(() => {
        if (!config?.features?.smart_paste) return [];
        const text = item.preview_text || item.content || "";
        if (item.content_type === "image") return [];
        return detectSmartContent(text);
    }, [item.id, item.preview_text, item.content, config?.features?.smart_paste]);

    // Sensitive content detection
    const isSensitive = useMemo(() => {
        if (!config?.features?.redact_sensitive) return false;
        const text = item.preview_text || item.content || "";
        if (item.content_type === "image") return false;
        return hasSensitiveContent(text);
    }, [item.id, item.preview_text, item.content, config?.features?.redact_sensitive]);

    // OCR handler for image items
    const handleOcr = async (e) => {
        e.stopPropagation();
        if (ocrLoading) return;
        setOcrLoading(true);
        try {
            const text = await invoke("extract_text_from_image", { imagePath: item.content });
            // Save OCR text as preview_text so it's searchable
            const updated = await invoke("update_preview_text", { id: item.id, previewText: text });
            if (onItemUpdate) onItemUpdate(updated);
            // Copy extracted text
            await invoke("paste_raw_text", { text });
            if (onToast) onToast(`✅ ${t("ocr.extracted")} — ${t("ocr.copy_text")}`);
        } catch (err) {
            if (onToast) onToast(`❌ ${String(err)}`);
        } finally {
            setOcrLoading(false);
        }
    };

    // Smart chip click handler
    const handleChipClick = async (e, chip) => {
        e.stopPropagation();
        const { action } = chip;
        if (action.type === "mailto") {
            window.open(action.url);
        } else if (action.type === "open") {
            window.open(action.url, "_blank");
        } else if (action.type === "copy") {
            await invoke("paste_raw_text", { text: action.value });
            if (onToast) onToast(`✅ Copied ${chip.label}`);
        } else if (action.type === "color") {
            await invoke("paste_raw_text", { text: action.value });
            if (onToast) onToast(`✅ Copied color ${action.value}`);
        }
    };

    const handleClick = (e) => {
        if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            if (onSelectToggle) onSelectToggle(item.id);
            return;
        }
        if (e.shiftKey) {
            e.preventDefault();
            if (onSelectRange) onSelectRange(index);
            return;
        }
        // Normal click: if in multi-select mode, toggle; otherwise paste
        if (isMultiSelect) {
            if (onSelectToggle) onSelectToggle(item.id);
        } else {
            onPaste(item.id);
        }
    };

    return (
        <div
            ref={index === selectedIndex ? selectedRef : null}
            className={`clip-item ${index === selectedIndex ? "selected" : ""} ${
                item.pinned ? "pinned" : ""
            } ${isChecked ? "checked" : ""}`}
            onClick={handleClick}
            onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                if (onContextMenu) onContextMenu(e, item);
            }}
            role="option"
            aria-selected={index === selectedIndex}
            aria-label={`${getTypeIcon(item.content_type)} ${formatPreview(item.preview_text, 40)}`}
        >
            {/* Checkmark overlay for multi-select */}
            {(isMultiSelect || isChecked) && (
                <div className="check-overlay" onClick={(e) => { e.stopPropagation(); if (onSelectToggle) onSelectToggle(item.id); }}>
                    <span className={`check-box ${isChecked ? "check-box-checked" : ""}`}>
                        {isChecked ? "✓" : ""}
                    </span>
                </div>
            )}

            <div className="clip-item-body">
                <div className="clip-item-header">
                    <span className="clip-type-icon">{getTypeIcon(item.content_type)}</span>
                    <TypeBadge type={item.content_type} />
                    {item.pinned && <span className="pin-badge">📌</span>}
                    {isSensitive && <span className="sensitive-badge" title={t("security.sensitive_detected")}>🔒</span>}
                    {tags.length > 0 && (
                        <span className="tag-badges">
                            {tags.map((tg) => (
                                <span key={tg} className="tag-badge">{tg}</span>
                            ))}
                        </span>
                    )}
                    <span className="clip-time">{formatTime(item.created_at)}</span>
                </div>

                <div className="clip-content">
                    {item.content_type === "image" ? (
                        <div className="clip-image-preview">
                            {imgSrc ? (
                                <img
                                    src={imgSrc}
                                    alt={t("clipboard.image_preview")}
                                    className="clip-thumbnail"
                                    loading="lazy"
                                />
                            ) : (
                                <span className="image-dims">{item.preview_text}</span>
                            )}
                            <span className="clip-type-label">clipboard-image.png</span>
                            <button
                                className="ocr-btn"
                                onClick={handleOcr}
                                disabled={ocrLoading}
                                title={t("ocr.extract")}
                            >
                                {ocrLoading ? "⏳" : "📝"} {t("ocr.button_label")}
                            </button>
                        </div>
                    ) : item.content_type === "files" ? (
                        <p className="clip-text clip-text-files">
                            📁 {formatFilesPreview(item)}
                        </p>
                    ) : item.content_type === "uri" ? (
                        <p className="clip-text clip-text-uri">
                            🔗 {extractDomain(item.content)}
                        </p>
                    ) : (
                        <p className={`clip-text${isCode ? " clip-text-code" : ""}${isSensitive ? " clip-text-sensitive" : ""}`}>
                            {isSensitive ? redactText(item.preview_text) : formatPreview(item.preview_text)}
                        </p>
                    )}

                    {/* Smart paste detection chips */}
                    {smartChips.length > 0 && (
                        <div className="smart-chips">
                            {smartChips.map((chip) => (
                                <button
                                    key={chip.type}
                                    className={`smart-chip smart-chip-${chip.type}`}
                                    onClick={(e) => handleChipClick(e, chip)}
                                    title={chip.match}
                                >
                                    {chip.icon} {chip.label}
                                    {chip.type === "color" && (
                                        <span className="color-swatch" style={{ background: chip.match }} />
                                    )}
                                </button>
                            ))}
                        </div>
                    )}
                </div>

                <div className="clip-actions">
                    <button
                        className="action-btn pin-btn"
                        onClick={(e) => {
                            e.stopPropagation();
                            onPin(item.id);
                        }}
                        title={item.pinned ? t("clipboard.unpin") : t("clipboard.pin")}
                        aria-label={item.pinned ? t("clipboard.unpin") : t("clipboard.pin")}
                    >
                        {item.pinned ? "📌" : "📍"}
                    </button>
                    <button
                        className="action-btn delete-btn"
                        onClick={(e) => {
                            e.stopPropagation();
                            onDelete(item.id);
                        }}
                        title={t("clipboard.delete")}
                        aria-label={t("clipboard.delete")}
                    >
                        🗑️
                    </button>
                </div>
            </div>
        </div>
    );
}

export default ClipboardList;
