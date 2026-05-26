import { useState, useCallback } from "react";
import { HexColorPicker } from "react-colorful";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/index.jsx";

const presets = [
    "#ff0000", "#ff8800", "#ffff00", "#00ff00", "#0088ff",
    "#0000ff", "#8800ff", "#ff00ff", "#ff0088", "#000000",
    "#888888", "#ffffff", "#cc0000", "#00aa00", "#0066cc",
];

function hexToRgb(hex) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgb(${r}, ${g}, ${b})`;
}

function hexToHsl(hex) {
    let r = parseInt(hex.slice(1, 3), 16) / 255;
    let g = parseInt(hex.slice(3, 5), 16) / 255;
    let b = parseInt(hex.slice(5, 7), 16) / 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h, s, l = (max + min) / 2;
    if (max === min) {
        h = s = 0;
    } else {
        const d = max - min;
        s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
        switch (max) {
            case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
            case g: h = ((b - r) / d + 2) / 6; break;
            default: h = ((r - g) / d + 4) / 6; break;
        }
    }
    return `hsl(${Math.round(h * 360)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;
}

function ColorPicker({ onToast }) {
    const { t } = useTranslation();
    const [color, setColor] = useState("#1e88e5");
    const [format, setFormat] = useState("hex");
    const [history, setHistory] = useState(() => {
        try {
            return JSON.parse(localStorage.getItem("color_history") || "[]");
        } catch { return []; }
    });

    const formatColor = useCallback((c, fmt) => {
        if (fmt === "hex") return c;
        if (fmt === "rgb") return hexToRgb(c);
        if (fmt === "hsl") return hexToHsl(c);
        return c;
    }, []);

    const handleCopy = async () => {
        const text = formatColor(color, format);
        try {
            await invoke("paste_raw_text", { text });
            const updated = [text, ...history.filter(h => h !== text)].slice(0, 20);
            setHistory(updated);
            localStorage.setItem("color_history", JSON.stringify(updated));
            onToast(`🎨 Copied ${text}`);
        } catch {
            onToast("❌ Failed to copy color");
        }
    };

    const currentFormatted = formatColor(color, format);

    return (
        <div className="color-picker">
            <div className="color-picker-main">
                <HexColorPicker color={color} onChange={setColor} />
            </div>
            <div className="color-preview-row">
                <div className="color-swatch" style={{ background: color }} />
                <div className="color-value">{currentFormatted}</div>
            </div>
            <div className="color-controls">
                <select
                    className="color-format-select"
                    value={format}
                    onChange={(e) => setFormat(e.target.value)}
                >
                    <option value="hex">HEX</option>
                    <option value="rgb">RGB</option>
                    <option value="hsl">HSL</option>
                </select>
                <button className="color-copy-btn" onClick={handleCopy}>
                    Copy
                </button>
            </div>
            <div className="color-presets">
                {presets.map((c) => (
                    <button
                        key={c}
                        className={`color-preset-swatch${c === color ? " active" : ""}`}
                        style={{ background: c }}
                        onClick={() => setColor(c)}
                        title={c}
                    />
                ))}
            </div>
            {history.length > 0 && (
                <div className="color-history">
                    <div className="color-history-label">{t("emoji.recent")}</div>
                    <div className="color-history-swatches">
                        {history.map((c) => (
                            <button
                                key={c}
                                className="color-history-swatch"
                                style={{ background: c.startsWith("#") ? c : undefined }}
                                onClick={() => {
                                    if (c.startsWith("#")) setColor(c);
                                }}
                                title={c}
                            />
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}

export default ColorPicker;