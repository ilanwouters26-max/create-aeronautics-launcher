export function bytes(n: number | undefined | null): string {
  if (!n || n <= 0) return "0 o";
  const units = ["o", "Ko", "Mo", "Go", "To"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

export function bitrate(bps: number): string {
  if (!bps || bps <= 0) return "0 o/s";
  return `${bytes(bps)}/s`;
}

export function duration(seconds: number): string {
  if (!seconds || seconds < 0) return "0 s";
  const s = Math.floor(seconds);
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  const rm = m % 60;
  if (h < 24) return rm ? `${h} h ${rm.toString().padStart(2, "0")}` : `${h} h`;
  const d = Math.floor(h / 24);
  return `${d} j ${h % 24} h`;
}

export function minutes(total: number): string {
  return duration(total * 60);
}

export function dateTime(iso: string | undefined | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("fr-FR", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
}

export function time(iso: string | undefined | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
}

export function relative(iso: string | undefined | null): string {
  if (!iso) return "";
  const d = new Date(iso).getTime();
  if (Number.isNaN(d)) return iso;
  const diff = Math.round((Date.now() - d) / 1000);
  if (diff < 0) return `dans ${duration(-diff)}`;
  if (diff < 60) return "à l'instant";
  return `il y a ${duration(diff)}`;
}

export function percent(v: number): string {
  if (!Number.isFinite(v)) return "0 %";
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} %`;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function basenameOfDir(path: string): string {
  const parts = path.replace(/[\\/]+$/, "").split(/[\\/]/);
  return parts.pop() ?? path;
}

export function joinPath(dir: string, child: string): string {
  const sep = dir.includes("\\") && !dir.includes("/") ? "\\" : "/";
  return dir.replace(/[\\/]+$/, "") + sep + child;
}

export function todayAt(hhmm: string): string {
  const [h, m] = hhmm.split(":").map((x) => parseInt(x, 10));
  const d = new Date();
  d.setHours(h || 0, m || 0, 0, 0);
  if (d.getTime() < Date.now()) d.setDate(d.getDate() + 1);
  return toLocalIso(d);
}

/** ISO 8601 avec le décalage local (le backend parse du RFC3339). */
export function toLocalIso(d: Date): string {
  const pad = (n: number) => n.toString().padStart(2, "0");
  const off = -d.getTimezoneOffset();
  const sign = off >= 0 ? "+" : "-";
  const a = Math.abs(off);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}${sign}${pad(Math.floor(a / 60))}:${pad(a % 60)}`;
}
