import { defineConfig } from "vite";

// GitHub Pages serves project sites under https://<user>.github.io/<repo>/, so all
// emitted asset URLs (JS, CSS, and the wasm fetched via `new URL(..., import.meta.url)`)
// must be relative rather than rooted at "/". `base: "./"` makes the build work under
// any subpath — the Pages project subpath, a local `vite preview`, or a bare domain —
// without hard-coding the repo name.
export default defineConfig({
  base: "./",
});
