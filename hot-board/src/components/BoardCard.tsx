import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import type { BoardResult } from "../types";

/** 每页显示的条数。列表不再内部滚动，超出部分翻页查看。 */
const PAGE_SIZE = 10;

function formatTime(unix: number) {
  if (!unix) return "—";
  const d = new Date(unix * 1000);
  return d.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
}

type Props = {
  board: BoardResult;
  loading?: boolean;
};

export function BoardCard({ board, loading }: Props) {
  const [page, setPage] = useState(0);

  const pageCount = Math.max(1, Math.ceil(board.items.length / PAGE_SIZE));
  // 刷新后条目数可能变少，用夹取后的页码渲染，避免越界空白
  const current = Math.min(page, pageCount - 1);
  const start = current * PAGE_SIZE;
  const items = board.items.slice(start, start + PAGE_SIZE);

  return (
    <section className="board-card">
      <header className="board-card__head">
        <h2>{board.name}</h2>
        <span className="board-card__meta">
          {loading ? "刷新中…" : `更新 ${formatTime(board.updated_at)}`}
        </span>
      </header>

      {board.error && (
        <p className="board-card__warn">
          {board.from_cache ? "加载失败，显示缓存" : "加载失败"}：{board.error}
        </p>
      )}

      <ol className="board-card__list">
        {items.map((item) => (
          <li key={`${board.id}-${item.rank}`}>
            <button
              type="button"
              className="board-item"
              onClick={() => openUrl(item.url)}
              title={item.title}
            >
              <span className={`rank rank--${item.rank <= 3 ? item.rank : "n"}`}>
                {item.rank}
              </span>
              <span className="title">{item.title}</span>
              {item.hot && <span className="hot">{item.hot}</span>}
            </button>
          </li>
        ))}
        {!board.items.length && !loading && (
          <li className="empty">暂无数据</li>
        )}
      </ol>

      {pageCount > 1 && (
        <nav className="board-card__pager" aria-label={`${board.name}分页`}>
          <button
            type="button"
            className="pager__btn"
            onClick={() => setPage(current - 1)}
            disabled={current === 0}
            aria-label="上一页"
          >
            ‹
          </button>
          <span className="pager__info">
            {current + 1} / {pageCount}
          </span>
          <button
            type="button"
            className="pager__btn"
            onClick={() => setPage(current + 1)}
            disabled={current >= pageCount - 1}
            aria-label="下一页"
          >
            ›
          </button>
        </nav>
      )}
    </section>
  );
}
