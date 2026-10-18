import { useEffect, useState } from "react";
import type {
  ProxyMode,
  ProxySetting,
  ProxySettingsPayload,
  ProxySettingsUpdate,
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

export function SettingsPanel({ settings, busy, error, onSave, onClose }: Props) {
  // 用本地状态做即时交互，保存时才提交
  const [setting, setSetting] = useState<ProxySetting>("auto");
  const [proxyUrl, setProxyUrl] = useState("");
  const [sourceModes, setSourceModes] = useState<Record<string, ProxyMode>>({});
  const [sourceDefaults, setSourceDefaults] = useState<Record<string, ProxyMode>>({});
  const [sourceNames, setSourceNames] = useState<Record<string, string>>({});

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
