import { useState, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/index.jsx";
import emojiData from "../data/emojis.json";

/** Map the i18n keys to the JSON category names */
const CATEGORY_KEYS = {
    "Smileys & Emotion": "emoji.smileys_emotion",
    "People & Body": "emoji.people_body",
    "Animals & Nature": "emoji.animals",
    "Food & Drink": "emoji.food",
    "Travel & Places": "emoji.travel",
    "Activities": "emoji.activities",
    "Objects": "emoji.objects",
    "Symbols": "emoji.symbols",
    "Flags": "emoji.flags",
};

const RECENT_KEY = "linvclip_recent_emojis";
const MAX_RECENT = 24;

function getRecent() {
    try {
        return JSON.parse(localStorage.getItem(RECENT_KEY) || "[]");
    } catch {
        return [];
    }
}

function addRecent(emoji) {
    const arr = getRecent().filter((e) => e !== emoji);
    arr.unshift(emoji);
    localStorage.setItem(RECENT_KEY, JSON.stringify(arr.slice(0, MAX_RECENT)));
}

function EmojiPicker({ searchQuery, onToast }) {
    const { t } = useTranslation();
    const [recent, setRecent] = useState(getRecent);

    const handleClick = useCallback(async (emoji) => {
        try {
            const result = await invoke("type_text", { text: emoji });
            addRecent(emoji);
            setRecent(getRecent());
            if (result === "typed") {
                if (onToast) onToast("✅ " + t("clipboard.inserted"));
            } else {
                if (onToast) onToast("📋 " + t("clipboard.copied"));
            }
        } catch (err) {
            console.error("Failed to insert emoji:", err);
            if (onToast) onToast("❌ " + t("clipboard.copy_failed"));
        }
    }, [t, onToast]);

    const query = (searchQuery || "").toLowerCase().trim();

    const filteredData = useMemo(() => {
        if (!query) return null; // show all categories normally
        const results = [];
        for (const emojis of Object.values(emojiData)) {
            for (const item of emojis) {
                if (item.name.includes(query) || item.emoji.includes(query)) {
                    results.push(item);
                }
            }
        }
        return results;
    }, [query]);

    // Searching — show flat grid of results
    if (filteredData !== null) {
        if (filteredData.length === 0) {
            return (
                <div className="picker-empty">
                    <p>{t("emoji.no_results")}</p>
                </div>
            );
        }
        return (
            <div className="picker-scroll">
                <div className="emoji-grid">
                    {filteredData.map((item, i) => (
                        <button
                            key={i}
                            className="emoji-cell"
                            onClick={() => handleClick(item.emoji)}
                            title={item.name}
                            aria-label={item.name}
                        >
                            {item.emoji}
                        </button>
                    ))}
                </div>
            </div>
        );
    }

    // Default view — show recent + all categories
    return (
        <div className="picker-scroll">
            {recent.length > 0 && (
                <>
                    <h3 className="picker-category-header">{t("emoji.recent")}</h3>
                    <div className="emoji-grid">
                        {recent.map((emoji, i) => (
                            <button
                                key={"r" + i}
                                className="emoji-cell"
                                onClick={() => handleClick(emoji)}
                                title={emoji}
                                aria-label={emoji}
                            >
                                {emoji}
                            </button>
                        ))}
                    </div>
                </>
            )}

            {Object.entries(emojiData).map(([category, emojis]) => (
                <div key={category}>
                    <h3 className="picker-category-header">
                        {t(CATEGORY_KEYS[category] || category)}
                    </h3>
                    <div className="emoji-grid">
                        {emojis.map((item, i) => (
                            <button
                                key={i}
                                className="emoji-cell"
                                onClick={() => handleClick(item.emoji)}
                                title={item.name}
                                aria-label={item.name}
                            >
                                {item.emoji}
                            </button>
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
}

export default EmojiPicker;
