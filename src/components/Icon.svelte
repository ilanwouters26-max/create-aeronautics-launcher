<script lang="ts">
  const icons: Record<string, { d: string; fill?: boolean }> = {
    play: { d: "M8 5v14l11-7z", fill: true },
    stop: { d: "M6 6h12v12H6z", fill: true },
    refresh: { d: "M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6" },
    power: { d: "M12 2v10M18.4 6.6a9 9 0 1 1-12.8 0" },
    folder: { d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" },
    plus: { d: "M12 5v14M5 12h14" },
    minus: { d: "M5 12h14" },
    trash: { d: "M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14M10 10v7M14 10v7" },
    search: { d: "M11 4a7 7 0 1 1 0 14 7 7 0 0 1 0-14zM20 20l-3.5-3.5" },
    terminal: { d: "M4 17l6-5-6-5M12 19h8" },
    server: { d: "M4 4h16v6H4zM4 14h16v6H4zM8 7h.01M8 17h.01" },
    cube: { d: "M12 2l9 5v10l-9 5-9-5V7zM12 12l9-5M12 12L3 7M12 12v10" },
    zap: { d: "M13 2L4 14h7l-1 8 9-12h-7z" },
    settings: { d: "M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6" },
    alert: { d: "M12 9v4M12 17h.01M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" },
    bug: { d: "M9 9V7a3 3 0 0 1 6 0v2M12 20v-8M6 13H3M21 13h-3M4 20l3-3M20 20l-3-3M4 6l3 2M20 6l-3 2M12 20a5 5 0 0 1-5-5v-3a5 5 0 0 1 10 0v3a5 5 0 0 1-5 5z" },
    shield: { d: "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" },
    "chevron-right": { d: "M9 18l6-6-6-6" },
    "chevron-down": { d: "M6 9l6 6 6-6" },
    check: { d: "M20 6L9 17l-5-5" },
    x: { d: "M18 6L6 18M6 6l12 12" },
    upload: { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12" },
    download: { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" },
    clock: { d: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 6v6l4 2" },
    users: { d: "M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM23 21v-2a4 4 0 0 0-3-3.9M16 3.1a4 4 0 0 1 0 7.8" },
    activity: { d: "M22 12h-4l-3 9L9 3l-3 9H2" },
    external: { d: "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6M15 3h6v6M10 14L21 3" },
    eye: { d: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" },
    "eye-off": { d: "M17.9 17.9A10 10 0 0 1 12 20c-7 0-11-8-11-8a18 18 0 0 1 5.1-6M9.9 4.2A9 9 0 0 1 12 4c7 0 11 8 11 8a18 18 0 0 1-2.2 3.2M1 1l22 22M9.9 9.9a3 3 0 0 0 4.2 4.2" },
    copy: { d: "M20 9h-9a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-9a2 2 0 0 0-2-2zM5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" },
    info: { d: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 16v-4M12 8h.01" },
    moon: { d: "M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z" },
    layers: { d: "M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" },
    git: { d: "M6 3v12M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 9a9 9 0 0 1-9 9" },
    home: { d: "M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM9 22V12h6v10" },
    filter: { d: "M22 3H2l8 9.5V19l4 2v-8.5z" },
    edit: { d: "M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7M18.5 2.5a2.1 2.1 0 0 1 3 3L12 15l-4 1 1-4z" },
    ban: { d: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM4.9 4.9l14.2 14.2" },
    link: { d: "M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7" },
    file: { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6" },
    list: { d: "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" },
    monitor: { d: "M20 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2zM8 21h8M12 17v4" },
    send: { d: "M22 2L11 13M22 2l-7 20-4-9-9-4z" },
    globe: { d: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" },
    cpu: { d: "M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3M6 6h12v12H6zM9 9h6v6H9z" },
    wifi: { d: "M5 12.5a11 11 0 0 1 14 0M8.5 16a6 6 0 0 1 7 0M2 9a16 16 0 0 1 20 0M12 20h.01" },
    calendar: { d: "M3 5h18v16H3zM3 10h18M8 3v4M16 3v4" },
    pause: { d: "M6 4h4v16H6zM14 4h4v16h-4z", fill: true },
    skull: { d: "M12 2a8 8 0 0 0-8 8c0 3 1.6 5.2 4 6.4V20h8v-3.6c2.4-1.2 4-3.4 4-6.4a8 8 0 0 0-8-8zM9 11h.01M15 11h.01M10 20v2M14 20v2" },
    pickaxe: { d: "M14 10L4 20M14 10l3-3M14 10l-3-3M17 7a9 9 0 0 1 4 1M11 7a9 9 0 0 1 1-4M17 7l-6-6" },
  };

  let { name, size = 16, stroke = 1.8 }: { name: string; size?: number; stroke?: number } = $props();
  const icon = $derived(icons[name] ?? icons.info);
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={icon.fill ? "currentColor" : "none"}
  stroke={icon.fill ? "none" : "currentColor"}
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  style="flex: 0 0 auto"
>
  <path d={icon.d} />
</svg>
