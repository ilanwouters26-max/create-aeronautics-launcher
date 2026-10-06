// Zones de dépôt : le glisser-déposer natif de Tauri donne une position, on
// retrouve la zone visée avec elementFromPoint. Dans un navigateur, HTML5.

import { api, isTauri } from "./api";

type DropHandler = (paths: string[]) => void;

const zones = new Map<string, DropHandler>();

export const drag = $state({ active: false, overZone: "" });

export function registerDropZone(id: string, handler: DropHandler): () => void {
  zones.set(id, handler);
  return () => {
    zones.delete(id);
    if (drag.overZone === id) drag.overZone = "";
  };
}

function zoneAt(x: number, y: number): string {
  const el = document.elementFromPoint(x, y);
  const zone = el?.closest<HTMLElement>("[data-dropzone]");
  return zone?.dataset.dropzone ?? "";
}

let installed = false;

export async function installDragDrop(): Promise<void> {
  if (installed) return;
  installed = true;
  if (!isTauri) return;
  await api.onDragDrop((ev) => {
    if (ev.type === "enter" || ev.type === "over") {
      drag.active = true;
      drag.overZone = zoneAt(ev.x, ev.y);
    } else if (ev.type === "drop") {
      const zone = zoneAt(ev.x, ev.y);
      drag.active = false;
      drag.overZone = "";
      const handler = zones.get(zone);
      if (handler && ev.paths.length) handler(ev.paths);
    } else {
      drag.active = false;
      drag.overZone = "";
    }
  });
}

/** Attributs/handlers HTML5 pour le mode navigateur (sans chemins réels). */
export function browserDropProps(id: string) {
  if (isTauri) return {};
  return {
    ondragover: (e: DragEvent) => {
      e.preventDefault();
      drag.active = true;
      drag.overZone = id;
    },
    ondragleave: () => {
      drag.overZone = "";
    },
    ondrop: (e: DragEvent) => {
      e.preventDefault();
      drag.active = false;
      drag.overZone = "";
      const names = Array.from(e.dataTransfer?.files ?? []).map((f) => f.name);
      const handler = zones.get(id);
      if (handler && names.length) handler(names);
    },
  };
}
