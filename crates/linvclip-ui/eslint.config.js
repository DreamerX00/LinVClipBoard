import js from "@eslint/js";
import globals from "globals";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";

export default [
    { ignores: ["dist/**", "node_modules/**", "src-tauri/**"] },
    js.configs.recommended,
    {
        files: ["**/*.{js,jsx}"],
        plugins: { react, "react-hooks": reactHooks },
        languageOptions: {
            ecmaVersion: 2023,
            sourceType: "module",
            parserOptions: { ecmaFeatures: { jsx: true } },
            globals: { ...globals.browser, ...globals.es2021, ...globals.node },
        },
        settings: { react: { version: "detect" } },
        rules: {
            ...react.configs.recommended.rules,
            ...react.configs["jsx-runtime"].rules,
            "react-hooks/rules-of-hooks": "error",
            "react-hooks/exhaustive-deps": "warn",
            // This codebase uses JSDoc rather than prop-types.
            "react/prop-types": "off",
            "react/no-unescaped-entities": "off",
            "no-unused-vars": [
                "error",
                { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrors: "none" },
            ],
            "no-empty": ["error", { allowEmptyCatch: true }],
        },
    },
    {
        files: ["**/*.test.{js,jsx}", "src/test/**"],
        languageOptions: { globals: { ...globals.node } },
    },
];
