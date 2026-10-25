/** 把用户保存的顺序套到当前云端清单上。空 saved 表示跟随 catalog。 */
export function mergeSourceOrder(
  catalog: readonly string[],
  saved: readonly string[],
): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  const known = new Set<string>();
  for (const raw of catalog) {
    const id = raw.trim();
    if (id) known.add(id);
  }

  for (const raw of saved) {
    const id = raw.trim();
    if (!id || !known.has(id) || seen.has(id)) continue;
    seen.add(id);
    out.push(id);
  }
  for (const raw of catalog) {
    const id = raw.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    out.push(id);
  }
  return out;
}

/** 把下标 from 的项挪到 to。越界或原地返回 null，调用方就不用改状态。 */
export function moveItem(
  ids: readonly string[],
  from: number,
  to: number,
): string[] | null {
  if (from === to || from < 0 || to < 0 || from >= ids.length || to >= ids.length) {
    return null;
  }
  const next = ids.slice();
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}
