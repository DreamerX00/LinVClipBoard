import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useTranslation } from "../i18n/index.jsx";
import LinkCard from "./LinkCard.jsx";

/**
 * Detects the content category for preview rendering.
 * Returns: "url" | "json" | "markdown" | "code" | "text" | "image"
 */
function detectContentType(item) {
    if (!item) return "text";
    if (item.content_type === "image") return "image";

    const text = (item.preview_text || item.content || "").trim();
    if (!text) return "text";

    // URL
    if (/^https?:\/\/\S+$/i.test(text)) return "url";

    // JSON
    if ((text.startsWith("{") || text.startsWith("[")) && text.length > 2) {
        try { JSON.parse(text); return "json"; } catch {}
    }

    // Markdown heuristics: headers, bold, links, code blocks, lists
    const mdScore =
        (/^#{1,6}\s/m.test(text) ? 1 : 0) +
        (/\*\*.+\*\*/m.test(text) ? 1 : 0) +
        (/\[.+\]\(.+\)/m.test(text) ? 1 : 0) +
        (/^```/m.test(text) ? 1 : 0) +
        (/^[-*]\s/m.test(text) ? 1 : 0);
    if (mdScore >= 2) return "markdown";

    // Code heuristics
    const codePatterns = /[{};=>]|function |const |let |var |import |def |class |<\/?\w+>/;
    if (codePatterns.test(text) && text.split("\n").length > 2) return "code";

    return "text";
}

function PreviewPane({ item, onPaste, onToast, onItemUpdate }) {
    const { t } = useTranslation();
    const [highlightedHtml, setHighlightedHtml] = useState("");
    const [imgSrc, setImgSrc] = useState(null);
    const [detectedLang, setDetectedLang] = useState("");
    const [ocrText, setOcrText] = useState("");
    const [ocrLoading, setOcrLoading] = useState(false);

    const contentType = useMemo(() => detectContentType(item), [item]);
    const text = item ? (item.preview_text || item.content || "") : "";

    // Stats
    const lineCount = text ? text.split("\n").length : 0;
    const wordCount = text.trim() ? text.trim().split(/\s+/).length : 0;
    const charCount = text.length;

    // Fetch syntax highlighting for code / JSON
    useEffect(() => {
        if (!item) return;
        let cancelled = false;
        setHighlightedHtml("");
        setDetectedLang("");

        if (contentType === "code" || contentType === "json") {
            const lang = contentType === "json" ? "json" : undefined;

            // Detect language first
            invoke("detect_language", { code: text })
                .then((lang) => { if (!cancelled) setDetectedLang(lang); })
                .catch(() => {});

            invoke("highlight_code", { code: text, language: lang })
                .then((html) => { if (!cancelled) setHighlightedHtml(html); })
                .catch(() => {});
        }

        return () => { cancelled = true; };
    }, [item?.id, contentType]);

    // Reset OCR text on item change
    useEffect(() => {
        setOcrText("");
        setOcrLoading(false);
    }, [item?.id]);

    // Fetch image
    useEffect(() => {
        if (!item || item.content_type !== "image") { setImgSrc(null); return; }
        let cancelled = false;
        invoke("get_image_base64", { path: item.content })
            .then((src) => { if (!cancelled) setImgSrc(src); })
            .catch(() => { if (!cancelled) setImgSrc(null); });
        return () => { cancelled = true; };
    }, [item?.id, item?.content]);

    const handleOcr = async () => {
        if (!item || item.content_type !== "image" || ocrLoading) return;
        setOcrLoading(true);
        try {
            const extracted = await invoke("extract_text_from_image", { imagePath: item.content });
            setOcrText(extracted);
            // Update preview text so it's searchable via FTS
            const updated = await invoke("update_preview_text", { id: item.id, previewText: extracted });
            if (onItemUpdate) onItemUpdate(updated);
            if (onToast) onToast(`✅ ${t("ocr.extracted")}`);
        } catch (err) {
            if (onToast) onToast(`❌ ${String(err)}`);
        } finally {
            setOcrLoading(false);
        }
    };

    const handleCopyOcrText = async () => {
        try {
            await invoke("paste_raw_text", { text: ocrText });
            if (onToast) onToast(`✅ ${t("ocr.copy_text")}`);
        } catch {}
    };

    const handleCopy = () => {
        if (item) {
            onPaste(item.id);
        }
    };

    if (!item) {
        return (
            <div className="preview-pane preview-empty">
                <span className="preview-empty-icon">👁</span>
                <p>{t("preview.no_selection")}</p>
            </div>
        );
    }

    return (
        <div className="preview-pane">
            {/* Header */}
            <div className="preview-header">
                <div className="preview-stats">
                    {detectedLang && contentType === "code" && (
                        <span className="preview-lang">{detectedLang}</span>
                    )}
                    {contentType === "json" && <span className="preview-lang">JSON</span>}
                    {contentType !== "image" && (
                        <span className="preview-meta">
                            {lineCount} {t("preview.lines")} · {wordCount} {t("preview.words")}
                        </span>
                    )}
                </div>
                <div className="preview-actions">
                    {contentType === "url" && (
                        <a
                            href={text}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="preview-btn"
                            title={t("preview.open_external")}
                        >
                            🔗
                        </a>
                    )}
                    <button className="preview-btn" onClick={handleCopy} title={t("preview.copy")}>
                        📋 {t("preview.copy")}
                    </button>
                </div>
            </div>

            {/* Content */}
            <div className="preview-content">
                {contentType === "image" && (
                    <div className="preview-image">
                        {imgSrc ? (
                            <img src={imgSrc} alt="Clipboard image" />
                        ) : (
                            <div className="preview-image-placeholder">
                                <span>🖼️</span>
                                <span>{item.preview_text}</span>
                            </div>
                        )}
                        <div className="preview-ocr-section">
                            <button
                                className="preview-ocr-btn"
                                onClick={handleOcr}
                                disabled={ocrLoading}
                            >
                                {ocrLoading ? `⏳ ${t("ocr.extracting")}` : `📝 ${t("ocr.extract")}`}
                            </button>
                            {ocrText && (
                                <div className="preview-ocr-result">
                                    <div className="preview-ocr-header">
                                        <span>{t("ocr.extracted")}</span>
                                        <button className="preview-btn" onClick={handleCopyOcrText}>
                                            📋 {t("ocr.copy_text")}
                                        </button>
                                    </div>
                                    <pre className="preview-ocr-text">{ocrText}</pre>
                                </div>
                            )}
                        </div>
                    </div>
                )}

                {contentType === "url" && <LinkCard url={text} />}

                {contentType === "markdown" && (
                    <div className="preview-markdown">
                        <ReactMarkdown remarkPlugins={[remarkGfm]}>{text}</ReactMarkdown>
                    </div>
                )}

                {(contentType === "code" || contentType === "json") && (
                    <div className="preview-code">
                        {highlightedHtml ? (
                            <div dangerouslySetInnerHTML={{ __html: highlightedHtml }} />
                        ) : (
                            <pre className="sh-code">{text}</pre>
                        )}
                    </div>
                )}

                {contentType === "text" && (
                    <div className="preview-text">
                        <pre>{text}</pre>
                    </div>
                )}
            </div>
        </div>
    );
}

export default PreviewPane;
