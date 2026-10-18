import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useState } from "react";
import { checkUpdate, getCached, getSettings, refreshAll, setSettings } from "./api";
import { BoardCard } from "./components/BoardCard";
import { RefreshIcon, SettingsIcon } from "./components/icons";
import { SettingsPanel } from "./components/SettingsPanel";
import type { BoardResult, ProxySettingsPayload, ProxySettingsUpdate } from "./types";
import "./App.css";

function App() {
  const [boards, setBoards] = useState<BoardResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [initial, setInitial] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settings, setSettingsData] = useState<ProxySettingsPayload | null>(null);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [updateAvailable, setUpdateAvailable] = useState(false);

  const ordered = useMemo(() => boards, [boards]);
  const subtitle = ordered.length
    ? `${ordered.map((board) => board.name).join(" · ")} · 点击标题用浏览器打开`
    : "点击标题用浏览器打开";

  const load = useCallback(async (silent = false) => {
    if (!silent) setLoading(true);
    try {
      const data = await refreshAll();
      setBoards(data);
    } finally {
      setLoading(false);
      setInitial(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const cached = await getCached();
        if (!cancelled && cached.length) {
          setBoards(cached);
          setInitial(false);
        }
      } catch {
        /* first run */
      }
      if (!cancelled) await load(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [load]);

  // 启动时静默检查一次更新，有新版本就在设置按钮上点个红点；失败不打扰用户。
  useEffect(() => {
    let cancelled = false;
    checkUpdate()
      .then((info) => {
        if (!cancelled) setUpdateAvailable(info.hasUpdate);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const openSettings = useCallback(async () => {
    setSettingsError(null);
    try {
      const data = await getSettings();
      setSettingsData(data);
      setSettingsOpen(true);
    } catch (e) {
      setSettingsError(`加载设置失败：${String(e)}`);
      setSettingsOpen(true);
    }
  }, []);

  const saveSettings = useCallback(
    async (payload: ProxySettingsUpdate) => {
      setSettingsBusy(true);
      setSettingsError(null);
      try {
        const updated = await setSettings(payload);
        setSettingsData(updated);
        // 如果改了代理配置，立即刷新一次（让设置生效）
        await load(true);
        return true;
      } catch (e) {
        setSettingsError(`保存设置失败：${String(e)}`);
        return false;
      } finally {
        setSettingsBusy(false);
      }
    },
    [load],
  );

  return (
    <div className="app">
      <header className="toolbar">
        <div>
          <h1>非凡热点榜</h1>
          <p className="subtitle">{subtitle}</p>
        </div>
        <div className="toolbar__actions">
          <button
            type="button"
            className="btn-icon"
            onClick={() => load()}
            disabled={loading && !initial}
            title={loading ? "刷新中…" : "全部刷新"}
            aria-label={loading ? "刷新中" : "全部刷新"}
          >
            <RefreshIcon className={loading ? "icon icon--spin" : "icon"} />
          </button>
          <button
            type="button"
            className="btn-icon"
            onClick={openSettings}
            title={updateAvailable ? "代理设置 · 有新版本" : "代理设置"}
            aria-label={updateAvailable ? "代理设置（有新版本）" : "代理设置"}
          >
            <SettingsIcon className="icon" />
            {updateAvailable && <span className="btn-icon__dot" aria-hidden="true" />}
          </button>
        </div>
      </header>

      <main className="grid">
        {ordered.map((b) => (
          <BoardCard key={b.id} board={b} loading={loading} />
        ))}
      </main>

      <footer className="footer">
        © 2026 深圳市汤姆网络科技有限公司{" "}
        <button
          type="button"
          className="footer__link"
          onClick={() => openUrl("https://beian.miit.gov.cn/")}
        >
          粤ICP备14061718号
        </button>
      </footer>

      {settingsOpen && (
        <SettingsPanel
          settings={settings}
          busy={settingsBusy}
          error={settingsError}
          onSave={saveSettings}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </div>
  );
}

export default App;
