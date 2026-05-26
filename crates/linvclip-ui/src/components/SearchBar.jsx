import { useRef, useEffect } from "react";

function SearchBar({ value, onChange, placeholder, onFocus, onBlur, isRegex, onToggleRegex }) {
    const inputRef = useRef(null);

    useEffect(() => {
        if (inputRef.current) {
            inputRef.current.focus();
        }
    }, []);

    return (
        <div className="search-bar" role="search" aria-label="Search">
            <span className="search-icon" aria-hidden="true">🔍</span>
            <input
                ref={inputRef}
                type="text"
                className="search-input"
                placeholder={placeholder || "Search..."}
                value={value}
                onChange={(e) => onChange(e.target.value)}
                onFocus={onFocus}
                onBlur={onBlur}
                autoComplete="off"
                spellCheck="false"
                aria-label={placeholder || "Search"}
                role="searchbox"
            />
            <button
                className={`search-regex-toggle${isRegex ? " active" : ""}`}
                onClick={onToggleRegex}
                title={isRegex ? "Regex mode: ON" : "Regex mode: OFF"}
                aria-label="Toggle regex search"
            >
                .*
            </button>
            {value && (
                <button
                    className="search-clear"
                    onClick={() => onChange("")}
                    aria-label="Clear search"
                >
                    ✕
                </button>
            )}
        </div>
    );
}

export default SearchBar;