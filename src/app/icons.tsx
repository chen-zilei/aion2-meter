/** Stroke icons for the sidebar, drawn on a 16px grid. */
const PATHS = {
  live: "M1.5 8h3l2-5 3 10 2-5h3",
  history: "M2 8a6 6 0 1 0 1.8-4.3M2 2v3h3M8 5v3l2 1.5",
  tracker: "M3 8l3 3 7-7M3 13.5h10",
  timers: "M8 14.5a5.5 5.5 0 1 0 0-11 5.5 5.5 0 0 0 0 11zM8 6v3l2 1M6 1.5h4",
  settings:
    "M8 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4",
  setup: "M8 14.5a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13zM6.2 6.2a1.9 1.9 0 1 1 2.6 1.8c-.5.2-.8.6-.8 1.1M8 11.3v.2",
  reset: "M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2v3h-3",
  keyboard: "M3 4h10a1.5 1.5 0 0 1 1.5 1.5v5A1.5 1.5 0 0 1 13 12H3a1.5 1.5 0 0 1-1.5-1.5v-5A1.5 1.5 0 0 1 3 4zM4 7h1M7 7h2M11 7h1M5 9.5h6",
};

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={PATHS[name]} />
    </svg>
  );
}
