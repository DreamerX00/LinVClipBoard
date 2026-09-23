/**
 * Smart content detection — identifies content types in clipboard text
 * and returns actionable badges/chips.
 */

const PATTERNS = [
    {
        type: "email",
        label: "Email",
        icon: "📧",
        regex: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/,
        action: (match) => ({ type: "mailto", url: `mailto:${match}` }),
    },
    {
        type: "phone",
        label: "Phone",
        icon: "📞",
        regex: /(\+?\d{1,3}[-.\s]?)?\(?\d{2,4}\)?[-.\s]?\d{3,4}[-.\s]?\d{4}\b/,
        action: (match) => ({ type: "copy", value: match }),
    },
    {
        type: "url",
        label: "URL",
        icon: "🔗",
        regex: /^https?:\/\/\S+$/i,
        action: (match) => ({ type: "open", url: match }),
    },
    {
        type: "color",
        label: "Color",
        icon: "🎨",
        regex: /#([0-9A-Fa-f]{3}){1,2}\b/,
        action: (match) => ({ type: "color", value: match }),
    },
    {
        type: "json",
        label: "JSON",
        icon: "{ }",
        regex: /^[\s]*[{[]/,
        validate: (text) => {
            try { JSON.parse(text); return true; } catch { return false; }
        },
        action: () => ({ type: "prettify" }),
    },
    {
        type: "filepath",
        label: "Path",
        icon: "📂",
        regex: /^(\/[\w.-]+)+\/?$/m,
        action: (match) => ({ type: "open_file", path: match }),
    },
    {
        type: "ip",
        label: "IP",
        icon: "🌐",
        regex: /\b(?:\d{1,3}\.){3}\d{1,3}\b/,
        action: (match) => ({ type: "copy", value: match }),
    },
    {
        type: "date",
        label: "Date",
        icon: "📅",
        regex: /\b\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}:\d{2})?/,
        action: (match) => ({ type: "date", value: match }),
    },
];

/**
 * Detect smart content types in text.
 * @param {string} text
 * @returns {Array<{type: string, label: string, icon: string, match: string, action: object}>}
 */
export function detectSmartContent(text) {
    if (!text || text.length > 10000) return [];

    const results = [];
    for (const pattern of PATTERNS) {
        const m = text.match(pattern.regex);
        if (m) {
            if (pattern.validate && !pattern.validate(text)) continue;
            results.push({
                type: pattern.type,
                label: pattern.label,
                icon: pattern.icon,
                match: m[0],
                action: pattern.action(m[0]),
            });
        }
    }
    return results;
}

/**
 * Sensitive content patterns for redaction.
 */
const SENSITIVE_PATTERNS = [
    { type: "credit_card", regex: /\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b/ },
    { type: "ssn", regex: /\b\d{3}-\d{2}-\d{4}\b/ },
    { type: "api_key", regex: /\b(sk|pk|api|key|token|secret|password)[_-]?\w{16,}\b/i },
    { type: "aws_key", regex: /\bAKIA[A-Z0-9]{16}\b/ },
    { type: "private_key", regex: /-----BEGIN (RSA |EC )?PRIVATE KEY-----/ },
    { type: "jwt", regex: /\beyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b/ },
];

/**
 * Check if text contains sensitive content.
 * @param {string} text
 * @returns {boolean}
 */
export function hasSensitiveContent(text) {
    if (!text) return false;
    return SENSITIVE_PATTERNS.some((p) => p.regex.test(text));
}

/**
 * Redact sensitive portions of text for display.
 * @param {string} text
 * @param {number} maxLen - max length for preview
 * @returns {string}
 */
export function redactText(text, maxLen = 80) {
    if (!text) return "";
    let result = text;

    // Credit card: show first 4 and last 4
    result = result.replace(
        /\b(\d{4})[\s-]?\d{4}[\s-]?\d{4}[\s-]?(\d{4})\b/g,
        "$1 •••• •••• $2"
    );

    // SSN
    result = result.replace(/\b\d{3}-\d{2}-\d{4}\b/g, "•••-••-••••");

    // API keys — keep prefix, mask rest
    result = result.replace(
        /\b((?:sk|pk|api|key|token|secret|password)[_-]?)\w{16,}\b/gi,
        "$1••••••••••••"
    );

    // AWS keys
    result = result.replace(/\bAKIA[A-Z0-9]{16}\b/g, "AKIA••••••••••••••••");

    // JWT — show first part only
    result = result.replace(
        /\beyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b/g,
        "eyJ•••.eyJ•••.•••"
    );

    // Private key
    result = result.replace(
        /-----BEGIN (RSA |EC )?PRIVATE KEY-----[\s\S]*?-----END (RSA |EC )?PRIVATE KEY-----/g,
        "-----BEGIN PRIVATE KEY----- [REDACTED] -----END PRIVATE KEY-----"
    );

    const clean = result.replace(/\n/g, " ↵ ").replace(/\s+/g, " ").trim();
    if (clean.length > maxLen) return clean.slice(0, maxLen - 3) + "...";
    return clean;
}
