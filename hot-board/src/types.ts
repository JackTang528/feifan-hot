export interface HotItem {
  rank: number;
  title: string;
  url: string;
  hot?: string;
}

export interface BoardResult {
  id: string;
  name: string;
  items: HotItem[];
  updated_at: number;
  error?: string;
  from_cache: boolean;
}

/** 云端更新清单的检查结果。 */
export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  hasUpdate: boolean;
  /** 安装包直链。 */
  url?: string;
  /** 下载页（没有直链时用）。 */
  pageUrl?: string;
  notes?: string;
  publishedAt?: string;
  manifestUrl: string;
}

export type ProxySetting = "auto" | "manual" | "off";
export type ProxyMode = "auto" | "proxy" | "direct";

/** 设置面板读回/写入的完整设置（只读字段除外）。 */
export interface ProxySettingsPayload {
  setting: ProxySetting;
  /** 手动代理地址，setting=manual 时使用。 */
  proxyUrl?: string;
  /** 用户手动改过的源。缺省 = 跟随云端默认。 */
  sourceModes: Record<string, ProxyMode>;
  /** 云端 boards.json 里配置的默认模式（只读展示）。 */
  sourceDefaults: Record<string, ProxyMode>;
  /** 榜单 id → 显示名称（只读，前端用于设置面板展示）。 */
  sourceNames: Record<string, string>;
  /** 云端 boards.json 的榜单 id 顺序。 */
  catalogOrder: string[];
  /** 用户自定义顺序。空数组表示跟随云端顺序。 */
  sourceOrder: string[];
  /** 当前 Windows 系统检测到的代理（提示用）。 */
  systemProxy?: string;
  /** setting=manual 时，手动代理客户端是否已构建成功。 */
  manualClientActive: boolean;
}

/** 提交给后端写入的设置（不含只读字段）。 */
export type ProxySettingsUpdate = Pick<
  ProxySettingsPayload,
  "setting" | "proxyUrl" | "sourceModes" | "sourceOrder"
>;
