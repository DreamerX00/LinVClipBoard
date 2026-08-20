import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-shell";
import { useTranslation } from "../i18n/index.jsx";

/**
 * UpdateModal — shown when a new version is available.
 *
 * Flow: idle → downloading → ready_to_install → installing → installed / error
 *
 * Props:
 *   updateInfo  – the UpdateInfo object from check_for_updates
 *   onClose     – callback to dismiss the modal
 */
function UpdateModal({ updateInfo, onClose }) {
    const { t } = useTranslation();
    // "idle" | "downloading" | "ready_to_install" | "installing" | "installed" | "error"
    const [stage, setStage] = useState("idle");
    const [progress, setProgress] = useState({ downloaded: 0, total: 0, percent: 0 });
    const [savedPath, setSavedPath] = useState("");
    const [errorMsg, setErrorMsg] = useState("");

    /* Listen to download-progress events from the Rust backend */
    useEffect(() => {
        let unlisten;
        (async () => {
            unlisten = await listen("download-progress", (event) => {
                setProgress(event.payload);
            });
        })();
        return () => { if (unlisten) unlisten(); };
    }, []);

    const isWindows = navigator.userAgent.includes("Windows");

    const handleDownload = useCallback(async () => {
        if (isWindows) {
            // Backend emits download-progress during download, then the app
            // exits to run the installer — "installed" is only reached on
            // paths where the plugin returns instead of relaunching.
            setStage("downloading");
            setProgress({ downloaded: 0, total: 0, percent: 0 });
            setErrorMsg("");
            try {
                await invoke("install_update_via_plugin");
                setStage("installed");
            } catch (err) {
                setErrorMsg(String(err));
                setStage("error");
            }
            return;
        }
        setStage("downloading");
        setProgress({ downloaded: 0, total: 0, percent: 0 });
        try {
            const path = await invoke("download_update", {
                url: updateInfo.download_url,
                version: updateInfo.latest_version,
            });
            setSavedPath(path);
            setStage("ready_to_install");
        } catch (err) {
            setErrorMsg(String(err));
            setStage("error");
        }
    }, [updateInfo, isWindows]);

    const handleInstall = useCallback(async () => {
        setStage("installing");
        setErrorMsg("");
        try {
            await invoke("install_update", { path: savedPath });
            setStage("installed");
        } catch (err) {
            const msg = String(err);
            if (msg.includes("auth_cancelled")) {
                setStage("ready_to_install");
            } else {
                setErrorMsg(msg);
                setStage("error");
            }
        }
    }, [savedPath]);

    const handleVisitGithub = useCallback(async () => {
        try {
            await open(updateInfo.release_url);
        } catch (_) {
            window.open(updateInfo.release_url, "_blank");
        }
    }, [updateInfo]);

    /* Format bytes for display */
    const fmtBytes = (b) => {
        if (b < 1024) return `${b} B`;
        if (b < 1048576) return `${(b / 1024).toFixed(1)} KB`;
        return `${(b / 1048576).toFixed(1)} MB`;
    };

    /**
     * Render simple Markdown-ish release notes using safe React elements.
     * Supports: ## headings, ### headings, - bullets, **bold**, `code`.
     * No dangerouslySetInnerHTML — immune to XSS from malicious release notes.
     */
    const renderNotes = (md) => {
        if (!md) return null;

        /** Parse inline markdown (bold + code) into React nodes. */
        const parseInline = (text) => {
            const parts = [];
            // Match **bold** and `code` spans
            const regex = /\*\*(.+?)\*\*|`(.+?)`/g;
            let lastIndex = 0;
            let match;
            while ((match = regex.exec(text)) !== null) {
                if (match.index > lastIndex) {
                    parts.push(text.slice(lastIndex, match.index));
                }
                if (match[1] !== undefined) {
                    parts.push(<b key={match.index}>{match[1]}</b>);
                } else if (match[2] !== undefined) {
                    parts.push(<code key={match.index}>{match[2]}</code>);
                }
                lastIndex = regex.lastIndex;
            }
            if (lastIndex < text.length) {
                parts.push(text.slice(lastIndex));
            }
            return parts.length > 0 ? parts : [text];
        };

        return md.split("\n").map((line, i) => {
            if (line.startsWith("### ")) return <h4 key={i}>{parseInline(line.slice(4))}</h4>;
            if (line.startsWith("## ")) return <h3 key={i}>{parseInline(line.slice(3))}</h3>;
            if (line.startsWith("- ")) return <li key={i}>{parseInline(line.slice(2))}</li>;
            if (!line.trim()) return <div key={i} className="update-notes-spacer" />;
            return <p key={i}>{parseInline(line)}</p>;
        });
    };

    /* ── Ready-to-install popup (appears after download) ── */
    if (stage === "ready_to_install" || stage === "installing" || stage === "installed") {
        return (
            <div className="update-modal-overlay">
                <div
                    className="update-modal update-install-modal"
                    role="dialog"
                    aria-modal="true"
                    onClick={(e) => e.stopPropagation()}
                >
                    {stage === "ready_to_install" && (
                        <>
                            <div className="update-install-header">
                                <span className="update-install-icon">📦</span>
                                <h2 className="update-modal-title">{t("update.ready_to_install")}</h2>
                            </div>
                            <div className="update-install-body">
                                <p className="update-install-file">
                                    {savedPath.split("/").pop()}
                                    <span className="update-install-size">
                                        {progress.total > 0 ? ` (${fmtBytes(progress.total)})` : ""}
                                    </span>
                                </p>
                                <p className="update-install-desc">{t("update.install_desc")}</p>
                            </div>
                            <div className="update-modal-actions">
                                <button className="update-btn-install" onClick={handleInstall}>
                                    🚀 {t("update.install_now")}
                                </button>
                                <button className="update-btn-secondary" onClick={handleVisitGithub}>
                                    🌐 {t("update.visit_github")}
                                </button>
                                <button className="update-btn-ghost" onClick={onClose}>
                                    {t("update.install_later")}
                                </button>
                            </div>
                        </>
                    )}

                    {stage === "installing" && (
                        <>
                            <div className="update-install-header">
                                <div className="update-installing-spinner" />
                                <h2 className="update-modal-title">{t("update.installing")}</h2>
                            </div>
                            <div className="update-install-body">
                                <p className="update-install-desc">{t("update.installing_desc")}</p>
                                <p className="update-install-hint">{t("update.installing_restart_hint")}</p>
                            </div>
                        </>
                    )}

                    {stage === "installed" && (
                        <>
                            <div className="update-install-header">
                                <span className="update-install-icon">🎉</span>
                                <h2 className="update-modal-title">{t("update.installed")}</h2>
                            </div>
                            <div className="update-install-body">
                                <p className="update-install-success">{t("update.installed_desc")}</p>
                            </div>
                            <div className="update-modal-actions">
                                <button className="update-btn-primary" onClick={onClose}>
                                    {t("update.close")}
                                </button>
                            </div>
                        </>
                    )}
                </div>
            </div>
        );
    }

    /* ── Main modal (idle / downloading / error) ── */
    return (
        <div className="update-modal-overlay" onClick={onClose}>
            <div
                className="update-modal"
                role="dialog"
                aria-modal="true"
                aria-label={t("update.title")}
                onClick={(e) => e.stopPropagation()}
            >
                {/* Header */}
                <div className="update-modal-header">
                    <span className="update-modal-icon">🚀</span>
                    <div>
                        <h2 className="update-modal-title">{t("update.title")}</h2>
                        <span className="update-modal-version">
                            v{updateInfo.current_version} → v{updateInfo.latest_version}
                        </span>
                    </div>
                </div>

                {/* Release notes */}
                <div className="update-notes-scroll">
                    <div className="update-notes">
                        {renderNotes(updateInfo.release_notes)}
                    </div>
                </div>

                {/* Progress bar */}
                {stage === "downloading" && (
                    <div className="update-progress-section">
                        <div className="update-progress-track">
                            <div
                                className="update-progress-fill"
                                style={{ width: `${progress.percent}%` }}
                            />
                        </div>
                        <div className="update-progress-text">
                            {fmtBytes(progress.downloaded)} / {progress.total > 0 ? fmtBytes(progress.total) : "…"}{" "}
                            <span className="update-progress-pct">{progress.percent.toFixed(0)}%</span>
                        </div>
                    </div>
                )}

                {stage === "error" && (
                    <div className="update-error-msg">⚠️ {errorMsg}</div>
                )}

                {/* Action buttons */}
                <div className="update-modal-actions">
                    {stage === "idle" && (
                        <>
                            <button className="update-btn-primary" onClick={handleDownload} disabled={!isWindows && !updateInfo.download_url}>
                                ⬇ {t("update.download_now")}
                            </button>
                            <button className="update-btn-secondary" onClick={handleVisitGithub}>
                                🌐 {t("update.visit_github")}
                            </button>
                            <button className="update-btn-ghost" onClick={onClose}>
                                {t("update.cancel")}
                            </button>
                        </>
                    )}
                    {stage === "downloading" && (
                        <button className="update-btn-ghost" disabled>
                            {t("update.downloading")}
                        </button>
                    )}
                    {stage === "error" && (
                        <>
                            <button className="update-btn-primary" onClick={handleDownload}>
                                🔄 {t("update.retry")}
                            </button>
                            <button className="update-btn-ghost" onClick={onClose}>
                                {t("update.close")}
                            </button>
                        </>
                    )}
                </div>
            </div>
        </div>
    );
}

export default UpdateModal;
