import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  root: "examples/codebase-observatory",
  base: "/examples/codebase-observatory/",
  plugins: [react()],
  build: {
    outDir: "../../dist/codebase-observatory",
    emptyOutDir: true,
  },
  test: {
    environment: "node",
  },
});
