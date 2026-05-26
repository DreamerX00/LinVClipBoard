import { useTranslation } from "../i18n/index.jsx";

const TABS = [
    { id: "clipboard", icon: "📋", labelKey: "tabs.clipboard" },
    { id: "emojis",    icon: "😀", labelKey: "tabs.emojis" },
    { id: "symbols",   icon: "Σ",  labelKey: "tabs.symbols" },
    { id: "gifs",      icon: "🎞️", labelKey: "tabs.gifs" },
    { id: "snippets",  icon: "📝", labelKey: "tabs.snippets" },
    { id: "colors",    icon: "🎨", labelKey: "tabs.colors" },
];

function TabBar({ activeTab, onTabChange }) {
    const { t } = useTranslation();

    return (
        <nav className="tab-bar" role="tablist" aria-label="Main tabs">
            {TABS.map((tab) => (
                <button
                    key={tab.id}
                    role="tab"
                    aria-selected={activeTab === tab.id}
                    aria-controls={`panel-${tab.id}`}
                    className={`tab-btn ${activeTab === tab.id ? "active" : ""}`}
                    onClick={() => onTabChange(tab.id)}
                >
                    <span className="tab-icon" aria-hidden="true">{tab.icon}</span>
                    <span className="tab-label">{t(tab.labelKey)}</span>
                </button>
            ))}
        </nav>
    );
}

export default TabBar;
