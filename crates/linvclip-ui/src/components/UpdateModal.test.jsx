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
    current_version: "3.2.1",
    latest_version: "3.2.2",
    release_url: "https://github.com/DreamerX00/LinVClipBoard/releases/tag/v3.2.2",
    release_notes: "## Fixed\n- things",
    download_url: "https://github.com/DreamerX00/LinVClipBoard/releases/download/v3.2.2/x",
    download_sha256: "abc123",
    via_plugin: false,
};

const calls = (cmd) => invoke.mock.calls.filter(([c]) => c === cmd);

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

beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation(() => new Promise(() => {})); // pending by default
});
afterEach(() => setUserAgent(originalUA));

describe("UpdateModal install routing", () => {
    it("Linux: downloads the package with its checksum, then installs via install_update", async () => {
        setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
        invoke.mockImplementation((cmd) => {
            if (cmd === "download_update") return Promise.resolve("/home/u/Downloads/linvclipboard_3.2.2_x86_64.deb");
            if (cmd === "install_update") return Promise.resolve("installed");
            return Promise.resolve(undefined);
        });
        renderModal(INFO);

        fireEvent.click(screen.getByRole("button", { name: new RegExp(en.update.download_now) }));
        await waitFor(() => expect(screen.getByText(en.update.ready_to_install)).toBeInTheDocument());

        expect(calls("download_update")).toHaveLength(1);
        expect(calls("download_update")[0][1]).toEqual({
            url: INFO.download_url,
            version: "3.2.2",
            sha256: "abc123",
        });
        expect(calls("install_update_via_plugin")).toHaveLength(0);
        expect(screen.getByText("linvclipboard_3.2.2_x86_64.deb")).toBeInTheDocument();
        expect(screen.getByText(en.update.install_desc)).toBeInTheDocument();

        fireEvent.click(screen.getByRole("button", { name: new RegExp(en.update.install_now) }));
        await waitFor(() => expect(calls("install_update")).toHaveLength(1));
        expect(calls("install_update")[0][1]).toEqual({ path: "/home/u/Downloads/linvclipboard_3.2.2_x86_64.deb" });
    });

    it("Windows with a signed manifest: installs through the updater plugin", async () => {
        setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        invoke.mockImplementation(() => new Promise(() => {}));
        renderModal({ ...INFO, via_plugin: true, download_sha256: "" });

        fireEvent.click(screen.getByRole("button", { name: new RegExp(en.update.download_now) }));
        await waitFor(() => expect(calls("install_update_via_plugin")).toHaveLength(1));
        expect(calls("download_update")).toHaveLength(0);
        expect(screen.getByText(en.update.downloading)).toBeInTheDocument();
    });

    it("Windows without the plugin: downloads the installer and runs it via install_update", async () => {
        setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
        invoke.mockImplementation((cmd) => {
            if (cmd === "download_update") return Promise.resolve("C:\\Users\\u\\Downloads\\linvclipboard_3.2.2_x86_64.exe");
            return new Promise(() => {});
        });
        renderModal({ ...INFO, via_plugin: false });

        fireEvent.click(screen.getByRole("button", { name: new RegExp(en.update.download_now) }));
        await waitFor(() => expect(screen.getByText(en.update.ready_to_install)).toBeInTheDocument());
        expect(calls("install_update_via_plugin")).toHaveLength(0);
        // Windows path separators are handled and the Windows copy is shown.
        expect(screen.getByText("linvclipboard_3.2.2_x86_64.exe")).toBeInTheDocument();
        expect(screen.getByText(en.update.install_desc_windows)).toBeInTheDocument();

        fireEvent.click(screen.getByRole("button", { name: new RegExp(en.update.install_now) }));
        await waitFor(() => expect(calls("install_update")).toHaveLength(1));
        expect(screen.getByText(en.update.installing_desc_windows)).toBeInTheDocument();
    });

    it("disables Download when there is nothing to download and no plugin", () => {
        setUserAgent("Mozilla/5.0 (X11; Linux aarch64)");
        renderModal({ ...INFO, download_url: "", via_plugin: false });
        expect(screen.getByRole("button", { name: new RegExp(en.update.download_now) })).toBeDisabled();
    });

    it("shows the backend error and offers Retry when the download fails", async () => {
        setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
        invoke.mockImplementation(() => Promise.reject("Checksum mismatch — the downloaded file was discarded"));
        renderModal(INFO);

        await act(async () => {
            fireEvent.click(screen.getByRole("button", { name: new RegExp(en.update.download_now) }));
        });
        await waitFor(() => expect(screen.getByText(/Checksum mismatch/)).toBeInTheDocument());
        expect(screen.getByRole("button", { name: new RegExp(en.update.retry) })).toBeInTheDocument();
    });
});
