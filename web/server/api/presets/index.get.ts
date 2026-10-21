import presets from "~~/shared/presets.json";
import { defineEventHandler, setHeader } from "h3";

/// The plans the API can return. A plan holds the vertical metrics and the
/// spacing the outlines were drawn at. Both come from the same file the page
/// reads, so a preset cannot mean one thing here and another in the builder.
export default defineEventHandler((event) => {
  setHeader(event, "cache-control", "public, max-age=3600");
  return {
    presets: presets.map(({ id, label, note, plan }) => ({
      id,
      label,
      note,
      plan,
      ttf: `/api/presets/${id}`,
      woff2: `/api/presets/${id}?format=woff2`,
    })),
  };
});
