import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { join, relative, sep } from "node:path";

const root = process.cwd();
const outDir = join(root, "dist");
const publicEntries = [
  "404.html",
  "index.html",
  "favicon.svg",
  "assets",
  "animations",
  "adr",
  "comparisons",
  "concepts",
  "demos",
  "docs",
  "examples",
  "guide",
  "roadmap",
];

rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

for (const entry of publicEntries) {
  const source = join(root, entry);
  if (!existsSync(source)) continue;
  cpSync(source, join(outDir, entry), {
    recursive: true,
    filter: (path) => {
      const rel = relative(root, path).split(sep).join("/");
      return rel !== "examples/codebase-observatory" && !rel.startsWith("examples/codebase-observatory/");
    },
  });
}

execFileSync(
  "vite",
  [
    "build",
    "--config",
    "examples/codebase-observatory/vite.config.ts",
    "--outDir",
    "../../dist/examples/codebase-observatory",
  ],
  { cwd: root, stdio: "inherit" },
);

console.log("Built static site into dist/");
