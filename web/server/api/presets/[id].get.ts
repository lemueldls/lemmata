import presets from "~~/shared/presets.json";
import { createError, defineEventHandler, getQuery, getRouterParam } from "h3";

const TYPES = {
  ttf: "font/ttf",
  woff2: "font/woff2",
} as const;

/// Sends one preset font. The TTF is the default. Add `?format=woff2` for the
/// compressed copy. Both are built by `scripts/presets.mjs` and bundled into
/// the worker as assets.
export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, "id");
  const preset = presets.find((candidate) => candidate.id === id);
  if (preset === undefined) {
    throw createError({ statusCode: 404, statusMessage: "Unknown preset" });
  }

  const format = getQuery(event).format === "woff2" ? "woff2" : "ttf";
  const stored = await useStorage("assets:server").getItemRaw(
    `presets/${preset.id}.${format}`,
  );
  if (!stored) {
    throw createError({
      statusCode: 404,
      statusMessage: "Preset font is missing",
    });
  }

  const body =
    stored instanceof Uint8Array
      ? stored
      : new Uint8Array(stored as ArrayBuffer);
  const bytes = body.buffer.slice(
    body.byteOffset,
    body.byteOffset + body.byteLength,
  ) as ArrayBuffer;
  return new Response(bytes, {
    headers: {
      "content-type": TYPES[format],
      "content-disposition": `attachment; filename="lemmata-${preset.id}.${format}"`,
      "cache-control": "public, max-age=3600",
    },
  });
});
