# Plan: Block Coders on the web (GitHub Pages)

Status: plan only, nothing built yet. These are the decisions and the work
needed when we're ready.

## The key fact: the game stays its own wasm file

macroquad compiles to `wasm32-unknown-unknown` and runs in a `<canvas>` using
its own small JavaScript loader (`mq_js_bundle.js`). It does **not** use
`wasm-bindgen`. Leptos *does* use `wasm-bindgen`, and Svelte is plain JS. So in
either case the game can't simply be "imported" as a component. Instead it is
built separately into three static files:

```
play/
  index.html          <canvas id="glcanvas"> + loader script
  mq_js_bundle.js     macroquad's loader (vendored, version-matched to miniquad 0.4)
  block-coders.wasm   the game (~1–2 MB)
```

The website then **embeds `play/index.html` in an `<iframe>`**. The iframe is
the recommended approach because:

- `mq_js_bundle.js` assumes it owns the page: one global canvas, global
  keyboard/mouse handlers, and no teardown. In a single-page app (Leptos router
  or SvelteKit), leaving the game page and coming back would try to start a
  second copy. An iframe gives the game a clean page every time.
- Keyboard focus, fullscreen, and canvas sizing stay simple.
- The site and the game deploy and version independently.

## Leptos or Svelte?

Since the game is an iframe either way, the framework only builds the *site
around* it: a landing page, a "for parents" guide, maybe a level gallery.

| | Leptos (CSR + Trunk) | SvelteKit (adapter-static) |
|---|---|---|
| Language | All Rust, which fits the "this game is written in Rust!" story | JS/TS |
| Tooling | `trunk build`, a wasm-bindgen site | `npm run build`, the most mature static tooling |
| GitHub Pages base path | `trunk build --public-url /<repo>/` | `paths.base = '/<repo>'` in config |
| Effort for a small site | Medium | Low |

**Recommendation:** If the site is just a few pages, Leptos is the nicer fit.
It keeps one language and the kids can see a whole Rust project. Choose
SvelteKit if the site will grow into lots of content or interactive pages. The
game side below is identical either way.

## Changes needed in the game

1. **Saving. (Done.)** Browsers have no file system, so `src/save.rs` has two
   `storage` backends chosen at compile time: files on desktop, and
   `localStorage` on `wasm32` through the miniquad plugin in `web/storage.js`
   (load it after `gl.js`). Keys are `block-coders/users.txt` and
   `block-coders/user-<id>.txt`. Saves never leave the device: no network,
   cookies or third parties. Everything loaded is treated as untrusted (anyone
   can edit localStorage): names are re-cleaned, ids and shirts validated,
   stars clamped, programs size- and depth-limited, and oversized values ignored.
   If storage is blocked (some privacy modes) the game still runs, it just
   doesn't remember progress. Saves are per browser; sharing between desktop and
   web would need an "export/import save code" button later.
2. **Native-only code.** Put `--write-icon`, `--version` and the `BC_SHOT`
   screenshot autopilot (in `main.rs`) behind `cfg(not(target_arch = "wasm32"))`.
3. **Crisp rendering.** Consider `high_dpi: true` in `window_conf` so text is
   sharp on high-DPI screens. The game already letterboxes to any canvas size.
4. **Audio.** Browsers only allow sound after a click. The title screen's PLAY
   button is that click, so nothing extra should be needed. Check this in
   Chrome and Firefox.
5. **Tablets and phones (optional).** macroquad turns taps into mouse clicks,
   so blocks and buttons work. Two gaps:
   - Typing a player name: tapping a canvas doesn't open the on-screen
     keyboard. Fix with a row of letter buttons in the New Player dialog, or by
     having the site pass the name in through `postMessage`.
   - Right-click delete: there's no right click on touch screens. The `x`
     button on each block already covers this.

   The layout is 1280×800, scaled down: fine on tablets in landscape, too small
   on phones.

## Build and deploy (GitHub Actions to GitHub Pages)

One workflow builds both parts and publishes them together:

```yaml
# .github/workflows/pages.yml (sketch)
on: { push: { branches: [main] } }
permissions: { contents: read, pages: write, id-token: write }
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - run: cargo test                                   # level checks still run on native
      - run: cargo build --release --target wasm32-unknown-unknown
      # Site: Leptos example (for SvelteKit: npm ci && npm run build)
      - run: cargo install trunk --locked && cd site && trunk build --release --public-url /${{ github.event.repository.name }}/
      - run: |
          mkdir -p site/dist/play
          cp web/index.html web/mq_js_bundle.js site/dist/play/
          cp target/wasm32-unknown-unknown/release/block-coders.wasm site/dist/play/
          touch site/dist/.nojekyll
      - uses: actions/upload-pages-artifact@v3
        with: { path: site/dist }
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment: github-pages
    steps:
      - uses: actions/deploy-pages@v4
```

And the site embeds the game with:

```html
<iframe src="play/index.html" width="1280" height="800"
        style="max-width:100%; aspect-ratio:16/10; border:0" allow="fullscreen"></iframe>
```

Notes:

- GitHub Pages serves `.wasm` with the right MIME type, so no extra setup is
  needed.
- Use relative paths (`play/...`, `load("block-coders.wasm")`) so it works
  under the `/<repo>/` subpath.
- To shrink the download, add `wasm-opt -Oz` (from binaryen) after the build.
  It usually saves 20–40%.

## Rough effort

- Game changes (saving, cfg gates, testing in browsers): about half a day.
- Site shell plus CI workflow: about half a day for a simple landing page.
