import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import { useCallback, useEffect, useRef, useState } from "react";
import { checkUpdate } from "../api";
import { mergeSourceOrder, moveItem } from "../sourceOrder";
import type {
  ProxyMode,
  ProxySetting,
  ProxySettingsPayload,
  ProxySettingsUpdate,
  UpdateInfo,
} from "../types";
import { GripIcon } from "./icons";

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
  const [setting, setSetting] = useState<ProxySetting>(settings?.setting ?? "auto");
  const [proxyUrl, setProxyUrl] = useState(settings?.proxyUrl ?? "");
  const [sourceModes, setSourceModes] = useState<Record<string, ProxyMode>>(
    settings?.sourceModes ?? {},
  );
  // null = 还没在这次打开里改过顺序，保存时沿用设置里的原始值（可能是空，表示跟随云端）。
  const [orderOverride, setOrderOverride] = useState<string[] | null>(null);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);
  // 拖动开始后马上就能放下，不能等 setState 再渲染。
  const draggingRef = useRef<string | null>(null);
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
    setOrderOverride(null);
  }, [settings]);

  function catalogIdsOf(current: ProxySettingsPayload): string[] {
    return current.catalogOrder.length > 0
      ? current.catalogOrder
      : Object.keys(current.sourceNames);
  }

  function moveSource(id: string, delta: number) {
    if (!settings) return;
    const catalogIds = catalogIdsOf(settings);
    setOrderOverride((prev) => {
      const ids = mergeSourceOrder(catalogIds, prev ?? settings.sourceOrder);
      const index = ids.indexOf(id);
      return moveItem(ids, index, index + delta) ?? prev;
    });
  }

  function dropSource(fromId: string, toId: string) {
    if (!settings || !fromId || fromId === toId) return;
    const catalogIds = catalogIdsOf(settings);
    setOrderOverride((prev) => {
      const ids = mergeSourceOrder(catalogIds, prev ?? settings.sourceOrder);
      return moveItem(ids, ids.indexOf(fromId), ids.indexOf(toId)) ?? prev;
    });
  }

  async function handleSave() {
    if (!settings) return;
    const payload: ProxySettingsUpdate = {
      setting,
      proxyUrl: proxyUrl.trim() || undefined,
      sourceModes,
      sourceOrder: orderOverride ?? settings.sourceOrder,
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

  const visibleIds = mergeSourceOrder(
    catalogIdsOf(settings),
    orderOverride ?? settings.sourceOrder,
  );

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
              默认跟随云端配置。「本地覆盖」会盖过云端值。右侧可上移、下移或拖动把手，调整主界面卡片的顺序。覆盖和顺序都写在本机，重启后仍然保留。
            </p>
            {visibleIds.length === 0 ? (
              <p className="settings__empty">暂无榜单。</p>
            ) : (
              <table className="settings__table">
                <thead>
                  <tr>
                    <th>榜单</th>
                    <th>云端默认</th>
                    <th>本地覆盖</th>
                    <th className="settings__sort">排序</th>
                  </tr>
                </thead>
                <tbody
                  onDragLeave={(e) => {
                    const next = e.relatedTarget;
                    if (next instanceof Node && e.currentTarget.contains(next)) return;
                    setOverId(null);
                  }}
                >
                  {visibleIds.map((id, index) => {
                    const serverDefault = settings.sourceDefaults[id] ?? "auto";
                    const userMode = sourceModes[id] ?? serverDefault;
                    const name = settings.sourceNames[id] ?? id;
                    const rowClass = [
                      draggingId === id ? "is-dragging" : "",
                      overId === id && draggingId && draggingId !== id ? "is-drop-target" : "",
                    ]
                      .filter(Boolean)
                      .join(" ");
                    return (
                      <tr
                        key={id}
                        data-source-id={id}
                        className={rowClass || undefined}
                        onDragOver={(e) => {
                          if (!draggingRef.current) return;
                          e.preventDefault();
                          e.dataTransfer.dropEffect = "move";
                          if (overId !== id) setOverId(id);
                        }}
                        onDrop={(e) => {
                          e.preventDefault();
                          const fromId =
                            e.dataTransfer.getData("text/plain") || draggingRef.current || "";
                          draggingRef.current = null;
                          dropSource(fromId, id);
                          setDraggingId(null);
                          setOverId(null);
                        }}
                      >
                        <td>{name}</td>
                        <td>{MODE_LABELS[serverDefault]}</td>
                        <td>
                          <select
                            aria-label={`${name}的本地覆盖`}
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
                        <td className="settings__sort">
                          <div className="settings__sort-actions">
                            <button
                              type="button"
                              className="sort-btn"
                              aria-label={`上移${name}`}
                              title="上移"
                              disabled={index === 0}
                              onClick={() => moveSource(id, -1)}
                            >
                              ↑
                            </button>
                            <button
                              type="button"
                              className="sort-btn"
                              aria-label={`下移${name}`}
                              title="下移"
                              disabled={index === visibleIds.length - 1}
                              onClick={() => moveSource(id, 1)}
                            >
                              ↓
                            </button>
                            <span
                              role="button"
                              tabIndex={0}
                              className="sort-btn sort-btn--handle"
                              draggable
                              aria-label={`拖动调整${name}的顺序`}
                              title="拖动排序"
                              onKeyDown={(e) => {
                                if (e.key === "ArrowUp") {
                                  e.preventDefault();
                                  moveSource(id, -1);
                                } else if (e.key === "ArrowDown") {
                                  e.preventDefault();
                                  moveSource(id, 1);
                                }
                              }}
                              onDragStart={(e) => {
                                draggingRef.current = id;
                                e.dataTransfer.effectAllowed = "move";
                                e.dataTransfer.setData("text/plain", id);
                                const row = e.currentTarget.closest("tr");
                                if (row) e.dataTransfer.setDragImage(row, 24, 16);
                                setDraggingId(id);
                              }}
                              onDragEnd={() => {
                                draggingRef.current = null;
                                setDraggingId(null);
                                setOverId(null);
                              }}
                            >
                              <GripIcon />
                            </span>
                          </div>
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
