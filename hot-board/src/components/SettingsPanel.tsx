import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import { useCallback, useEffect, useState } from "react";
import { checkUpdate } from "../api";
import type {
  ProxyMode,
  ProxySetting,
  ProxySettingsPayload,
  ProxySettingsUpdate,
  UpdateInfo,
} from "../types";

interface Props {
  settings: ProxySettingsPayload | null;
  busy: boolean;
  error: string | null;
  onSave: (payload: ProxySettingsUpdate) => Promise<boolean>;
  onClose: () => void;
}

const SETTING_LABELS: Record<ProxySetting, string> = {
  auto: "跟随系统代理",
  manual: "手动指定",
  off: "完全关闭",
};

const MODE_LABELS: Record<ProxyMode, string> = {
  auto: "跟随全局",
  proxy: "走代理",
  direct: "直连",
};

type UpdateState =
  | { status: "checking" }
  | { status: "error"; message: string }
  | { status: "done"; info: UpdateInfo };

export function SettingsPanel({ settings, busy, error, onSave, onClose }: Props) {
  // 用本地状态做即时交互，保存时才提交
  const [setting, setSetting] = useState<ProxySetting>("auto");
  const [proxyUrl, setProxyUrl] = useState("");
  const [sourceModes, setSourceModes] = useState<Record<string, ProxyMode>>({});
  const [sourceDefaults, setSourceDefaults] = useState<Record<string, ProxyMode>>({});
  const [sourceNames, setSourceNames] = useState<Record<string, string>>({});
  const [version, setVersion] = useState("");
  const [updateState, setUpdateState] = useState<UpdateState>({ status: "checking" });

  const runCheck = useCallback(async () => {
    setUpdateState({ status: "checking" });
    try {
      const info = await checkUpdate();
      setUpdateState({ status: "done", info });
    } catch (e) {
      setUpdateState({ status: "error", message: String(e) });
    }
  }, []);

  // 打开设置就自动检查一次；版本号直接问 Tauri，失败就留空。
  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => {});
    void runCheck();
  }, [runCheck]);

  useEffect(() => {
    if (!settings) return;
    setSetting(settings.setting);
    setProxyUrl(settings.proxyUrl ?? "");
    setSourceModes(settings.sourceModes ?? {});
    setSourceDefaults(settings.sourceDefaults ?? {});
    setSourceNames(settings.sourceNames ?? {});
  }, [settings]);

  async function handleSave() {
    const payload = {
      setting,
      proxyUrl: proxyUrl.trim() || undefined,
      sourceModes,
    };
    await onSave(payload);
  }

  if (!settings) {
    return (
      <div className="modal-backdrop" onClick={onClose}>
        <div className="modal" onClick={(e) => e.stopPropagation()}>
          <header className="modal__head">
            <h2>代理设置</h2>
            <button className="btn-close" onClick={onClose} aria-label="关闭">×</button>
          </header>
          {error ? <p className="settings__error">{error}</p> : <p>加载中…</p>}
        </div>
      </div>
    );
  }

  const sortedIds = Object.keys(sourceNames).sort((a, b) => {
    const ai = sourceDefaults[a] ?? "auto";
    const bi = sourceDefaults[b] ?? "auto";
    // 先显示云端的默认代理，再显示直连，最后跟随全局
    const order: Record<ProxyMode, number> = { proxy: 0, auto: 1, direct: 2 };
    return order[ai] - order[bi];
  });

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <header className="modal__head">
          <h2>代理设置</h2>
          <button className="btn-close" onClick={onClose} aria-label="关闭">×</button>
        </header>

        <div className="modal__body">
          {error && <p className="settings__error">{error}</p>}

          <section className="settings__section">
            <div className="settings__section-head">
              <h3>软件更新</h3>
              <button
                type="button"
                className="update__check"
                onClick={() => void runCheck()}
                disabled={updateState.status === "checking"}
              >
                {updateState.status === "checking" ? "检查中…" : "重新检查"}
              </button>
            </div>

            <p className="settings__hint">
              当前版本{" "}
              <code>
                v
                {updateState.status === "done"
                  ? updateState.info.currentVersion
                  : version || "…"}
              </code>
            </p>

            {updateState.status === "error" && (
              <p className="update__fail">
                暂时无法检查更新：{updateState.message}
              </p>
            )}

            {updateState.status === "done" && !updateState.info.hasUpdate && (
              <p className="update__ok">
                已是最新版本（云端 v{updateState.info.latestVersion}）。
              </p>
            )}

            {updateState.status === "done" && updateState.info.hasUpdate && (
              <>
                <p className="update__title">
                  发现新版本 v{updateState.info.latestVersion}
                </p>
                {updateState.info.publishedAt && (
                  <p className="settings__hint">
                    发布于 {updateState.info.publishedAt}
                  </p>
                )}
                {updateState.info.notes && (
                  <pre className="update__notes">{updateState.info.notes}</pre>
                )}
                {updateState.info.url || updateState.info.pageUrl ? (
                  <>
                    <div className="update__actions">
                      <button
                        type="button"
                        className="btn-primary"
                        onClick={() =>
                          openUrl(
                            (updateState.info.url ?? updateState.info.pageUrl)!,
                          )
                        }
                      >
                        下载更新
                      </button>
                    </div>
                    <p className="settings__hint">
                      用浏览器下载安装包。装之前先退出本软件（包括托盘里的）。
                    </p>
                  </>
                ) : (
                  <p className="settings__hint">
                    更新清单里没有下载地址，请到官网手动下载。
                  </p>
                )}
              </>
            )}
          </section>

          <section className="settings__section">
            <h3>全局代理模式</h3>
            <div className="settings__radio">
              {(["auto", "manual", "off"] as const).map((mode) => (
                <label key={mode} className="radio">
                  <input
                    type="radio"
                    name="proxy-setting"
                    checked={setting === mode}
                    onChange={() => setSetting(mode)}
                  />
                  <span>{SETTING_LABELS[mode]}</span>
                </label>
              ))}
            </div>

            {setting === "auto" && (
              <p className="settings__hint">
                当前 Windows 系统代理：
                {settings.systemProxy ? (
                  <code>{settings.systemProxy}</code>
                ) : (
                  <em>（未检测到）</em>
                )}
                <br />
                以后启动会自动跟随。改系统代理后重启软件生效。
              </p>
            )}

            {setting === "manual" && (
              <>
                <p className="settings__hint">
                  {settings.manualClientActive
                    ? "手动代理客户端已就绪。"
                    : "手动代理客户端未生效（URL 可能无效或为空）。"}
                </p>
                <div className="settings__field">
                  <label htmlFor="proxy-url">代理地址</label>
                  <input
                    id="proxy-url"
                    type="text"
                    value={proxyUrl}
                    onChange={(e) => setProxyUrl(e.target.value)}
                    placeholder="http://127.0.0.1:7897"
                    spellCheck={false}
                  />
                  <p className="settings__hint">
                    支持 http / https / socks5。示例：<code>http://127.0.0.1:7897</code>
                  </p>
                </div>
              </>
            )}

            {setting === "off" && (
              <p className="settings__hint">
                所有榜单都直连，即使下面单独标了「走代理」也不会用。
              </p>
            )}
          </section>

          <section className="settings__section">
            <h3>每个源的开关</h3>
            <p className="settings__hint">
              默认跟随云端配置。下面的「本地覆盖」列会强制覆盖云端值，重启软件后仍保留。
            </p>
            {sortedIds.length === 0 ? (
              <p className="settings__empty">暂无榜单。</p>
            ) : (
              <table className="settings__table">
                <thead>
                  <tr>
                    <th>榜单</th>
                    <th>云端默认</th>
                    <th>本地覆盖</th>
                  </tr>
                </thead>
                <tbody>
                  {sortedIds.map((id) => {
                    const serverDefault = sourceDefaults[id] ?? "auto";
                    const userMode = sourceModes[id] ?? serverDefault;
                    return (
                      <tr key={id}>
                        <td>{sourceNames[id] ?? id}</td>
                        <td>{MODE_LABELS[serverDefault]}</td>
                        <td>
                          <select
                            value={userMode}
                            onChange={(e) => {
                              const next = { ...sourceModes, [id]: e.target.value as ProxyMode };
                              setSourceModes(next);
                            }}
                            disabled={setting === "off"}
                          >
                            <option value="auto">跟随全局</option>
                            <option value="proxy">走代理</option>
                            <option value="direct">直连</option>
                          </select>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
          </section>
        </div>

        <footer className="modal__foot">
          <button className="btn-secondary" onClick={onClose} disabled={busy}>
            取消
          </button>
          <button
            className="btn-primary"
            onClick={handleSave}
            disabled={busy || (setting === "manual" && !proxyUrl.trim())}
          >
            {busy ? "保存中…" : "保存并应用"}
          </button>
        </footer>
      </div>
    </div>
  );
}
