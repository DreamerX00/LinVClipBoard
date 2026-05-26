import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/index.jsx";

/**
 * Context menu for clipboard items.
 * Shows quick actions: copy, pin, delete, transforms, encode/decode, JSON, QR, tags.
 */
function ContextMenu({ item, x, y, onClose, onPin, onDelete, onPaste, onToast, onShowQr, onItemUpdate, onSaveAsSnippet }) {
    const { t } = useTranslation();
    const menuRef = useRef(null);
    const [openSub, setOpenSub] = useState(null); // which submenu is open
    const [tagInput, setTagInput] = useState("");
    const [showTagInput, setShowTagInput] = useState(false);
    const tagInputRef = useRef(null);

    // Position adjustment to stay in viewport (pre-compute to avoid visual jump)
    const [pos, setPos] = useState(() => {
        const vw = window.innerWidth;
        const vh = window.innerHeight;
        const pad = 6;
        const estW = 200;
        const estH = 400;
        let nx = x;
        let ny = y;
        if (nx + estW > vw - pad) nx = Math.max(pad, vw - estW - pad);
        if (ny + estH > vh - pad) ny = Math.max(pad, vh - estH - pad);
        return { x: nx, y: ny };
    });
    const [flipSub, setFlipSub] = useState(false);

    useEffect(() => {
        if (!menuRef.current) return;
        const rect = menuRef.current.getBoundingClientRect();
        const vw = window.innerWidth;
        const vh = window.innerHeight;
        const pad = 6;
        let nx = x;
        let ny = y;
        if (nx + rect.width > vw - pad) nx = Math.max(pad, vw - rect.width - pad);
        if (ny + rect.height > vh - pad) ny = Math.max(pad, vh - rect.height - pad);
        setPos({ x: nx, y: ny });
        setFlipSub(nx + rect.width + 160 > vw);
    }, [x, y]);

    // Close on outside click, escape, scroll
    useEffect(() => {
        const handleClose = (e) => {
            if (menuRef.current && !menuRef.current.contains(e.target)) onClose();
        };
        const handleKey = (e) => {
            if (e.key === "Escape") onClose();
        };
        const handleScroll = () => onClose();

        document.addEventListener("mousedown", handleClose);
        document.addEventListener("keydown", handleKey, true);
        window.addEventListener("scroll", handleScroll, true);
        window.addEventListener("blur", onClose);
        return () => {
            document.removeEventListener("mousedown", handleClose);
            document.removeEventListener("keydown", handleKey, true);
            window.removeEventListener("scroll", handleScroll, true);
            window.removeEventListener("blur", onClose);
        };
    }, [onClose]);

    // Focus tag input when shown
    useEffect(() => {
        if (showTagInput && tagInputRef.current) tagInputRef.current.focus();
    }, [showTagInput]);

    const isText = item.content_type !== "image";
    const isImage = item.content_type === "image";
    const [ocrLoading, setOcrLoading] = useState(false);
    const text = item.preview_text || item.content || "";

    // Parse tags
    let tags = [];
    try { tags = JSON.parse(item.tags || "[]"); } catch { tags = []; }

    // ─── Action helpers ───

    const copyTransformed = async (result, label) => {
        try {
            await invoke("paste_raw_text", { text: result });
            onToast(`✅ ${t("toast.copied_as")} ${label}`);
        } catch {
            onToast("❌ Failed");
        }
        onClose();
    };

    const handleCopy = () => { onPaste(item.id); onClose(); };
    const handlePin = () => { onPin(item.id); onClose(); };
    const handleDelete = () => { onDelete(item.id); onClose(); };

    // Transforms
    const handleUppercase = () => copyTransformed(text.toUpperCase(), "UPPERCASE");
    const handleLowercase = () => copyTransformed(text.toLowerCase(), "lowercase");
    const handleTitleCase = () => {
        const result = text.replace(/\w\S*/g, (w) => w[0].toUpperCase() + w.slice(1).toLowerCase());
        copyTransformed(result, "Title Case");
    };
    const handleTrim = () => copyTransformed(text.trim().replace(/\s+/g, " "), "Trimmed");

    // Encode/Decode
    const handleBase64Encode = () => {
        try {
            const result = btoa(unescape(encodeURIComponent(text)));
            copyTransformed(result, "Base64");
        } catch { onToast("❌ Encode failed"); onClose(); }
    };
    const handleBase64Decode = () => {
        try {
            const result = decodeURIComponent(escape(atob(text)));
            copyTransformed(result, "Base64 Decoded");
        } catch { onToast("❌ " + t("toast.invalid_base64")); onClose(); }
    };
    const handleUrlEncode = () => copyTransformed(encodeURIComponent(text), "URL Encoded");
    const handleUrlDecode = () => {
        try {
            copyTransformed(decodeURIComponent(text), "URL Decoded");
        } catch { onToast("❌ Decode failed"); onClose(); }
    };

    // JSON
    const handleJsonPrettify = () => {
        try {
            const result = JSON.stringify(JSON.parse(text), null, 2);
            copyTransformed(result, "JSON Prettified");
        } catch { onToast("❌ " + t("toast.invalid_json")); onClose(); }
    };
    const handleJsonMinify = () => {
        try {
            const result = JSON.stringify(JSON.parse(text));
            copyTransformed(result, "JSON Minified");
        } catch { onToast("❌ " + t("toast.invalid_json")); onClose(); }
    };

    // Word count
    const handleWordCount = () => {
        const count = text.trim() ? text.trim().split(/\s+/).length : 0;
        const chars = text.length;
        onToast(`📊 ${t("toast.word_count")}: ${count} words, ${chars} chars`);
        onClose();
    };

    // Format conversion detection
    const isJson = useCallback(() => {
        try { JSON.parse(text); return true; }
        catch { return false; }
    }, [text]);

    const looksLikeYaml = useCallback(() => {
        return /^[\w-]+:\s|^---\s/.test(text.trim());
    }, [text]);

    const looksLikeToml = useCallback(() => {
        return /^\[[\w.]+]|^\w+\s*=/.test(text.trim());
    }, [text]);

    // Format converters
    const handleJsonToYaml = () => copyTransformedFromInvoke("convert_json_to_yaml", { jsonText: text }, "YAML");
    const handleYamlToJson = () => copyTransformedFromInvoke("convert_yaml_to_json", { yamlText: text }, "JSON");
    const handleJsonToToml = () => copyTransformedFromInvoke("convert_json_to_toml", { jsonText: text }, "TOML");
    const handleTomlToJson = () => copyTransformedFromInvoke("convert_toml_to_json", { tomlText: text }, "JSON");

    const copyTransformedFromInvoke = async (cmd, args, label) => {
        try {
            const result = await invoke(cmd, args);
            await invoke("paste_raw_text", { text: result });
            onToast(`✅ ${t("toast.copied_as")} ${label}`);
        } catch (err) {
            onToast(`❌ ${String(err)}`);
        }
        onClose();
    };

    // OCR
    const handleOcr = async () => {
        setOcrLoading(true);
        try {
            const text = await invoke("extract_text_from_image", { imagePath: item.content });
            await invoke("update_preview_text", { id: item.id, previewText: text });
            await invoke("paste_raw_text", { text });
            onToast(`✅ ${t("ocr.extracted")} — ${t("ocr.copy_text")}`);
        } catch (err) {
            onToast(`❌ ${String(err)}`);
        } finally {
            setOcrLoading(false);
        }
        onClose();
    };

    // QR Code
    const handleQrCode = () => {
        onShowQr(text);
        onClose();
    };

    // Tags
    const handleAddTag = async () => {
        const tag = tagInput.trim();
        if (!tag) return;
        try {
            const updated = await invoke("add_tag", { id: item.id, tag });
            if (onItemUpdate) onItemUpdate(updated);
            onToast(`🏷️ Tag "${tag}" added`);
        } catch (e) {
            onToast("❌ " + String(e));
        }
        setTagInput("");
        setShowTagInput(false);
        onClose();
    };

    const handleRemoveTag = async (tag) => {
        try {
            const updated = await invoke("remove_tag", { id: item.id, tag });
            if (onItemUpdate) onItemUpdate(updated);
            onToast(`🏷️ Tag "${tag}" removed`);
        } catch (e) {
            onToast("❌ " + String(e));
        }
        onClose();
    };

    return (
        <div
            className="context-menu"
            ref={menuRef}
            style={{ left: pos.x, top: pos.y }}
            role="menu"
        >
            {/* Primary actions */}
            <button className="ctx-item" onClick={handleCopy} role="menuitem">
                <span className="ctx-icon">📋</span> {t("context.copy")}
            </button>
            <button className="ctx-item" onClick={handlePin} role="menuitem">
                <span className="ctx-icon">{item.pinned ? "📌" : "📍"}</span>{" "}
                {item.pinned ? t("context.unpin") : t("context.pin")}
            </button>
            <button className="ctx-item ctx-danger" onClick={handleDelete} role="menuitem">
                <span className="ctx-icon">🗑️</span> {t("context.delete")}
            </button>

            <div className="ctx-separator" />

            {/* Transform submenu (text only) */}
            {isText && (
                <div
                    className={`ctx-submenu-wrap${openSub === "transform" ? " open" : ""}${flipSub ? " flip-left" : ""}`}
                    onMouseEnter={() => setOpenSub("transform")}
                    onMouseLeave={() => setOpenSub(null)}
                >
                    <button className="ctx-item ctx-has-sub" role="menuitem">
                        <span className="ctx-icon">✏️</span> {t("context.transform")} <span className="ctx-arrow">▸</span>
                    </button>
                    {openSub === "transform" && (
                        <div className="ctx-submenu" role="menu">
                            <button className="ctx-item" onClick={handleUppercase} role="menuitem">UPPERCASE</button>
                            <button className="ctx-item" onClick={handleLowercase} role="menuitem">lowercase</button>
                            <button className="ctx-item" onClick={handleTitleCase} role="menuitem">Title Case</button>
                            <button className="ctx-item" onClick={handleTrim} role="menuitem">{t("context.trim")}</button>
                        </div>
                    )}
                </div>
            )}

            {/* Encode/Decode submenu (text only) */}
            {isText && (
                <div
                    className={`ctx-submenu-wrap${openSub === "encode" ? " open" : ""}${flipSub ? " flip-left" : ""}`}
                    onMouseEnter={() => setOpenSub("encode")}
                    onMouseLeave={() => setOpenSub(null)}
                >
                    <button className="ctx-item ctx-has-sub" role="menuitem">
                        <span className="ctx-icon">🔐</span> {t("context.encode_decode")} <span className="ctx-arrow">▸</span>
                    </button>
                    {openSub === "encode" && (
                        <div className="ctx-submenu" role="menu">
                            <button className="ctx-item" onClick={handleBase64Encode} role="menuitem">Base64 Encode</button>
                            <button className="ctx-item" onClick={handleBase64Decode} role="menuitem">Base64 Decode</button>
                            <div className="ctx-separator" />
                            <button className="ctx-item" onClick={handleUrlEncode} role="menuitem">URL Encode</button>
                            <button className="ctx-item" onClick={handleUrlDecode} role="menuitem">URL Decode</button>
                        </div>
                    )}
                </div>
            )}

            {/* JSON submenu (text only) */}
            {isText && (
                <div
                    className={`ctx-submenu-wrap${openSub === "json" ? " open" : ""}${flipSub ? " flip-left" : ""}`}
                    onMouseEnter={() => setOpenSub("json")}
                    onMouseLeave={() => setOpenSub(null)}
                >
                    <button className="ctx-item ctx-has-sub" role="menuitem">
                        <span className="ctx-icon">{"{ }"}</span> {t("context.json")} <span className="ctx-arrow">▸</span>
                    </button>
                    {openSub === "json" && (
                        <div className="ctx-submenu" role="menu">
                            <button className="ctx-item" onClick={handleJsonPrettify} role="menuitem">{t("context.json_prettify")}</button>
                            <button className="ctx-item" onClick={handleJsonMinify} role="menuitem">{t("context.json_minify")}</button>
                        </div>
                    )}
                </div>
            )}

            {/* Convert submenu (text only, format-aware) */}
            {isText && (isJson() || looksLikeYaml() || looksLikeToml()) && (
                <div
                    className={`ctx-submenu-wrap${openSub === "convert" ? " open" : ""}${flipSub ? " flip-left" : ""}`}
                    onMouseEnter={() => setOpenSub("convert")}
                    onMouseLeave={() => setOpenSub(null)}
                >
                    <button className="ctx-item ctx-has-sub" role="menuitem">
                        <span className="ctx-icon">🔄</span> Convert <span className="ctx-arrow">▸</span>
                    </button>
                    {openSub === "convert" && (
                        <div className="ctx-submenu" role="menu">
                            {isJson() && <button className="ctx-item" onClick={handleJsonToYaml} role="menuitem">JSON → YAML</button>}
                            {isJson() && <button className="ctx-item" onClick={handleJsonToToml} role="menuitem">JSON → TOML</button>}
                            {looksLikeYaml() && <button className="ctx-item" onClick={handleYamlToJson} role="menuitem">YAML → JSON</button>}
                            {looksLikeToml() && <button className="ctx-item" onClick={handleTomlToJson} role="menuitem">TOML → JSON</button>}
                        </div>
                    )}
                </div>
            )}

            <div className="ctx-separator" />

            {/* OCR (image only) */}
            {isImage && (
                <button className="ctx-item" onClick={handleOcr} disabled={ocrLoading} role="menuitem">
                    <span className="ctx-icon">📝</span> {ocrLoading ? t("ocr.extracting") : t("context.ocr")}
                </button>
            )}

            {/* QR Code */}
            <button className="ctx-item" onClick={handleQrCode} role="menuitem">
                <span className="ctx-icon">📱</span> {t("context.qr_code")}
            </button>

            {/* Save as Snippet (text only) */}
            {isText && onSaveAsSnippet && (
                <button className="ctx-item" onClick={() => { onSaveAsSnippet(item); onClose(); }} role="menuitem">
                    <span className="ctx-icon">📝</span> {t("context.save_as_snippet")}
                </button>
            )}

            {/* Word Count (text only) */}
            {isText && (
                <button className="ctx-item" onClick={handleWordCount} role="menuitem">
                    <span className="ctx-icon">#️⃣</span> {t("context.word_count")}
                </button>
            )}

            <div className="ctx-separator" />

            {/* Add Tag */}
            {!showTagInput ? (
                <button className="ctx-item" onClick={() => setShowTagInput(true)} role="menuitem">
                    <span className="ctx-icon">🏷️</span> {t("context.add_tag")}
                </button>
            ) : (
                <div className="ctx-tag-input-row">
                    <input
                        ref={tagInputRef}
                        className="ctx-tag-input"
                        value={tagInput}
                        onChange={(e) => setTagInput(e.target.value)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") handleAddTag();
                            if (e.key === "Escape") { setShowTagInput(false); setTagInput(""); }
                        }}
                        placeholder="Tag name…"
                        maxLength={30}
                    />
                    <button className="ctx-tag-add-btn" onClick={handleAddTag}>+</button>
                </div>
            )}

            {/* Remove Tags (if item has tags) */}
            {tags.length > 0 && (
                <div
                    className={`ctx-submenu-wrap${openSub === "tags" ? " open" : ""}${flipSub ? " flip-left" : ""}`}
                    onMouseEnter={() => setOpenSub("tags")}
                    onMouseLeave={() => setOpenSub(null)}
                >
                    <button className="ctx-item ctx-has-sub" role="menuitem">
                        <span className="ctx-icon">🗑️</span> {t("context.remove_tags")} <span className="ctx-arrow">▸</span>
                    </button>
                    {openSub === "tags" && (
                        <div className="ctx-submenu" role="menu">
                            {tags.map((tg) => (
                                <button key={tg} className="ctx-item" onClick={() => handleRemoveTag(tg)} role="menuitem">
                                    ✕ {tg}
                                </button>
                            ))}
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}

export default ContextMenu;
