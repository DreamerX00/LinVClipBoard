import { useState, useEffect, useMemo } from "react";
import { useTranslation } from "../i18n/index.jsx";

function SnippetVarDialog({ snippet, onSubmit, onCancel }) {
    const { t } = useTranslation();

    // Extract variable names from content
    const varNames = useMemo(() => {
        const matches = [...snippet.content.matchAll(/\{\{(\w+)\}\}/g)];
        return [...new Set(matches.map((m) => m[1]))];
    }, [snippet.content]);

    // Parse defaults from snippet.variables JSON
    const defaults = useMemo(() => {
        try {
            const parsed = JSON.parse(snippet.variables || "[]");
            const map = {};
            for (const v of parsed) {
                if (v.name) map[v.name] = v.default ?? "";
            }
            return map;
        } catch {
            return {};
        }
    }, [snippet.variables]);

    const [values, setValues] = useState(() => {
        const init = {};
        for (const name of varNames) {
            init[name] = defaults[name] || "";
        }
        return init;
    });

    // Live preview
    const preview = useMemo(() => {
        let result = snippet.content;
        for (const [key, val] of Object.entries(values)) {
            result = result.replaceAll(`{{${key}}}`, val || `{{${key}}}`);
        }
        return result;
    }, [snippet.content, values]);

    const handleChange = (name, value) => {
        setValues((prev) => ({ ...prev, [name]: value }));
    };

    return (
        <div className="var-dialog-overlay" onClick={onCancel}>
            <div className="var-dialog" onClick={(e) => e.stopPropagation()}>
                <h3>{t("snippets.fill_variables")}</h3>

                <div className="var-fields">
                    {varNames.map((name) => (
                        <div key={name} className="var-field">
                            <label>{name}</label>
                            <input
                                type="text"
                                value={values[name] || ""}
                                onChange={(e) => handleChange(name, e.target.value)}
                                autoFocus={name === varNames[0]}
                                onKeyDown={(e) => {
                                    if (e.key === "Enter") onSubmit(values);
                                }}
                            />
                        </div>
                    ))}
                </div>

                <div className="var-preview-section">
                    <label>{t("snippets.preview")}</label>
                    <pre className="var-preview">{preview}</pre>
                </div>

                <div className="var-dialog-actions">
                    <button className="snippet-btn snippet-btn-cancel" onClick={onCancel}>
                        {t("snippets.cancel")}
                    </button>
                    <button className="snippet-btn snippet-btn-save" onClick={() => onSubmit(values)}>
                        {t("snippets.copy")}
                    </button>
                </div>
            </div>
        </div>
    );
}

export default SnippetVarDialog;
