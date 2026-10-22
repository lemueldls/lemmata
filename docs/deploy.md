# Deploying the site

The site deploys to Cloudflare Workers. `web/wrangler.jsonc` names the worker
and points at the built entry and static assets. The worker build uses
`NITRO_PRESET=cloudflare_module` and produces `.output/server/index.mjs` and
`.output/public`.

```sh
pnpm --dir web build:workers   # worker
pnpm --dir web preview         # build the worker and serve it on :8787
pnpm --dir web deploy          # build the worker and upload
```

`preview` and `deploy` call Wrangler, which needs a login once:
`pnpm --dir web exec wrangler login`.

The plain `pnpm --dir web build` still runs `nuxt generate`, which writes
`.output/public` for the browser suite and any static host.

## Preset API

The site build writes the preset fonts under `web/server/assets/presets`. The
directory is generated and is not stored in Git. Nitro bundles the files into
the worker, so a request does not need a compiler. The deploy build does need
the native toolchain that produces them.

| Route | Returns |
| --- | --- |
| `GET /api/presets` | The plans, with a TTF and a WOFF2 link for each |
| `GET /api/presets/<id>` | The preset as a TTF |
| `GET /api/presets/<id>?format=woff2` | The preset as a WOFF2 |

The plans live in `web/shared/presets.json`. Change a plan or a capture, then
rebuild the files with `pnpm --dir web presets:write`. The plain
`pnpm --dir web presets` only fills in missing files.

## GitHub Actions

`.github/workflows/deploy.yml` builds a pull request and deploys a push to
`main`. A manual run on `main` deploys too.

The job uses the Nix shell because the site build needs `cargo`: the preset
fonts are generated from the capture, and the browser package is built before
`pnpm` resolves the workspace. It also fetches the `vendor/Iosevka` submodule,
which holds the parameter files the build reads.

Set two repository secrets:

| Secret | Value |
| --- | --- |
| `CLOUDFLARE_API_TOKEN` | A token with Workers Scripts:Edit and Account Settings:Read |
| `CLOUDFLARE_ACCOUNT_ID` | The Cloudflare account id |

The workflow caches the Cargo target, the Cargo registry and the pnpm store. A
pull request builds the worker and runs `wrangler deploy --dry-run`; it does not
need the Cloudflare secrets.
