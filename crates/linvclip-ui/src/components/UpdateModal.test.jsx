import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, act, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { I18nProvider } from "../i18n/index.jsx";
import en from "../i18n/en.json";
import UpdateModal from "./UpdateModal.jsx";

vi.mock("@tauri-apps/api/event", () => ({
    listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("@tauri-apps/plugin-shell", () => ({
    open: vi.fn(() => Promise.resolve()),
}));

const INFO = {
    has_update: true,
    current_version: "3.3.0",
    latest_version: "3.3.1",
    release_url: "https://github.com/DreamerX00/LinVClipBoard/releases/tag/v3.3.1",
    release_notes: "## Fixed\n- things",
    download_url: "https://github.com/DreamerX00/LinVClipBoard/releases/download/v3.3.1/pkg",
    checksum_url: "https://github.com/DreamerX00/LinVClipBoard/releases/download/v3.3.1/SHA256SUMS",
};

const calls = (cmd) => invoke.mock.calls.filter(([c]) => c === cmd);
const button = (label) => screen.getByRole("button", { name: new RegExp(label) });

function setUserAgent(ua) {
    Object.defineProperty(window.navigator, "userAgent", { value: ua, configurable: true });
}

function renderModal(info, onClose = () => {}) {
    return render(
        <I18nProvider>
            <UpdateModal updateInfo={info} onClose={onClose} />
        </I18nProvider>
    );
}

const originalUA = window.navigator.userAgent;
const pending = () => new Promise(() => {});

beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation(pending);
});
afterEach(() => setUserAgent(originalUA));

describe("UpdateModal – install routing", () => {
    it("Linux: downloads the package with its checksum URL, then installs via install_update", async () => {
        setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
        invoke.mockImplementation((cmd) => {
            if (cmd === "download_update") return Promise.resolve("/home/u/Downloads/linvclipboard_3.3.1_x86_64.deb");
            if (cmd === "install_update") return Promise.resolve("installed");
            return Promise.resolve(undefined);
        });
        renderModal(INFO);

        fireEvent.click(button(en.update.download_now));
        await waitFor(() => expect(screen.getByText(en.update.ready_to_install)).toBeInTheDocument());

        expect(calls("install_update_via_plugin")).toHaveLength(0);
        expect(calls("download_update")).toHaveLength(1);
        expect(calls("download_update")[0][1]).toEqual({
            url: INFO.download_url,
            version: "3.3.1",
            checksumUrl: INFO.checksum_url,
        });
        expect(screen.getByText("linvclipboard_3.3.1_x86_64.deb")).toBeInTheDocument();

        fireEvent.click(button(en.update.install_now));
        await waitFor(() => expect(calls("install_update")).toHaveLength(1));
        expect(calls("install_update")[0][1]).toEqual({ path: "/home/u/Downloads/linvclipboard_3.3.1_x86_64.deb" });
        expect(screen.getByText(en.update.installed)).toBeInTheDocument();
    });

    it("Windows: installs through the signed updater plugin first", async () => {
        setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        renderModal(INFO);

        fireEvent.click(button(en.update.download_now));
        await waitFor(() => expect(calls("install_update_via_plugin")).toHaveLength(1));
        expect(calls("download_update")).toHaveLength(0);
        expect(screen.getByText(en.update.downloading)).toBeInTheDocument();
    });

    it("Windows: falls back to the verified installer download when the plugin is unavailable", async () => {
        setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        invoke.mockImplementation((cmd) => {
            if (cmd === "install_update_via_plugin") return Promise.reject("updater_unavailable: no signed manifest");
            if (cmd === "download_update") return Promise.resolve("C:\\Users\\u\\Downloads\\linvclipboard_3.3.1_x86_64.exe");
            return pending();
        });
        renderModal(INFO);

        fireEvent.click(button(en.update.download_now));
        await waitFor(() => expect(screen.getByText(en.update.ready_to_install)).toBeInTheDocument());
        expect(calls("install_update_via_plugin")).toHaveLength(1);
        expect(calls("download_update")).toHaveLength(1);
        expect(calls("download_update")[0][1].checksumUrl).toBe(INFO.checksum_url);
        // Windows path separators are handled in the file label.
        expect(screen.getByText("linvclipboard_3.3.1_x86_64.exe")).toBeInTheDocument();

        fireEvent.click(button(en.update.install_now));
        await waitFor(() => expect(calls("install_update")).toHaveLength(1));
    });

    it("Windows: a mid-install plugin failure is shown as an error, not retried via download", async () => {
        setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        invoke.mockImplementation((cmd) => {
            if (cmd === "install_update_via_plugin") return Promise.reject("signature verification failed");
            return pending();
        });
        renderModal(INFO);

        await act(async () => {
            fireEvent.click(button(en.update.download_now));
        });
        await waitFor(() => expect(screen.getByText(/signature verification failed/)).toBeInTheDocument());
        expect(calls("download_update")).toHaveLength(0);
        expect(button(en.update.retry)).toBeInTheDocument();
    });

    it("disables Download when there is no package for this system", () => {
        setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
        renderModal({ ...INFO, download_url: "" });
        expect(button(en.update.download_now)).toBeDisabled();
    });

    it("shows the backend error and offers Retry when the download fails", async () => {
        setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
        invoke.mockImplementation(() => Promise.reject("Checksum mismatch: the downloaded file was discarded"));
        renderModal(INFO);

        await act(async () => {
            fireEvent.click(button(en.update.download_now));
        });
        await waitFor(() => expect(screen.getByText(/Checksum mismatch/)).toBeInTheDocument());
        expect(button(en.update.retry)).toBeInTheDocument();
    });
});
