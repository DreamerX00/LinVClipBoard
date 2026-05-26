import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/index.jsx";

function SnippetEditor({ snippet, folders, onSave, onCancel, onToast }) {
    const { t } = useTranslation();
    const isEdit = !!snippet;

    const [name, setName] = useState(snippet?.name || "");
    const [content, setContent] = useState(snippet?.content || "");
    const [folder, setFolder] = useState(snippet?.folder || "");
    const [newFolder, setNewFolder] = useState("");
    const [abbreviation, setAbbreviation] = useState(snippet?.abbreviation || "");
    const [detectedVars, setDetectedVars] = useState([]);

    // Auto-detect {{variables}} in content
    useEffect(() => {
        const matches = [...content.matchAll(/\{\{(\w+)\}\}/g)];
        const unique = [...new Set(matches.map((m) => m[1]))];
        setDetectedVars(unique);
    }, [content]);

    const handleSave = async () => {
        if (!name.trim() || !content.trim()) return;

        const finalFolder = newFolder.trim() || folder;
        // Preserve existing variable defaults when editing, use "" for new vars
        const existingDefaults = (() => {
            try {
                const parsed = JSON.parse(snippet?.variables || "[]");
                const map = {};
                for (const v of parsed) if (v.name) map[v.name] = v.default ?? "";
                return map;
            } catch { return {}; }
        })();
        const variables = JSON.stringify(
            detectedVars.map((v) => ({ name: v, default: existingDefaults[v] ?? "" }))
        );

        try {
            if (isEdit) {
                await invoke("update_snippet", {
                    id: snippet.id,
                    name: name.trim(),
                    content,
                    folder: finalFolder,
                    abbreviation: abbreviation.trim(),
                    variables,
                });
            } else {
                await invoke("create_snippet", {
                    name: name.trim(),
                    content,
                    folder: finalFolder,
                    abbreviation: abbreviation.trim(),
                    variables,
                });
            }
            onToast(isEdit ? t("snippets.updated") : t("snippets.created"));
            onSave();
        } catch (err) {
            console.error("Save snippet failed:", err);
            onToast("Failed to save snippet");
        }
    };

    return (
        <div className="snippet-editor">
            <h3 className="snippet-editor-title">
                {isEdit ? t("snippets.edit") : t("snippets.new")}
            </h3>

            <div className="snippet-field">
                <label>{t("snippets.name")}</label>
                <input
                    type="text"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="Email Greeting"
                    autoFocus
                />
            </div>

            <div className="snippet-field">
                <label>{t("snippets.abbreviation")}</label>
                <input
                    type="text"
                    value={abbreviation}
                    onChange={(e) => setAbbreviation(e.target.value)}
                    placeholder="/greet"
                />
            </div>

            <div className="snippet-field">
                <label>{t("snippets.folder")}</label>
                <div className="snippet-folder-select">
                    <select
                        value={folder}
                        onChange={(e) => setFolder(e.target.value)}
                    >
                        <option value="">—</option>
                        {folders.map((f) => (
                            <option key={f} value={f}>
                                {f}
                            </option>
                        ))}
                    </select>
                    <input
                        type="text"
                        value={newFolder}
                        onChange={(e) => setNewFolder(e.target.value)}
                        placeholder={t("snippets.new_folder")}
                        className="snippet-new-folder-input"
                    />
                </div>
            </div>

            <div className="snippet-field snippet-field-content">
                <label>{t("snippets.content")}</label>
                <textarea
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    rows={8}
                    placeholder={"Hello {{name}},\n\nRegarding {{project}}..."}
                />
            </div>

            {detectedVars.length > 0 && (
                <div className="snippet-vars-detected">
                    <label>{t("snippets.variables")}</label>
                    <div className="snippet-var-tags">
                        {detectedVars.map((v) => (
                            <span key={v} className="snippet-var-tag">
                                {`{{${v}}}`}
                            </span>
                        ))}
                    </div>
                </div>
            )}

            <div className="snippet-editor-actions">
                <button className="snippet-btn snippet-btn-cancel" onClick={onCancel}>
                    {t("snippets.cancel")}
                </button>
                <button
                    className="snippet-btn snippet-btn-save"
                    onClick={handleSave}
                    disabled={!name.trim() || !content.trim()}
                >
                    {t("snippets.save")}
                </button>
            </div>
        </div>
    );
}

export default SnippetEditor;
