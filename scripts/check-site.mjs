import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join, normalize } from "node:path";

const root = process.cwd();
const requiredRoutes = [
  "/",
  "/guide",
  "/guide/why-resource-graphs",
  "/guide/core-model",
  "/guide/transactions",
  "/guide/scopes",
  "/guide/testing",
  "/examples",
  "/examples/local-first-sync",
  "/examples/language-server",
  "/examples/iot-dashboard",
  "/examples/plugin-runtime",
  "/examples/market-data",
  "/examples/nostr-feed",
  "/concepts",
  "/comparisons",
  "/comparisons/signals",
  "/comparisons/rx",
  "/comparisons/salsa",
  "/comparisons/dataflow",
  "/comparisons/controllers",
  "/docs",
  "/roadmap"
];

function walk(dir) {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    if (entry === ".git" || entry === "node_modules") return [];
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

function routePath(route) {
  return route === "/" ? join(root, "index.html") : join(root, route, "index.html");
}

const htmlFiles = walk(root).filter((path) => extname(path) === ".html");
const errors = [];

for (const route of requiredRoutes) {
  if (!existsSync(routePath(route))) errors.push(`Missing route: ${route}`);
}

for (const file of htmlFiles) {
  const html = readFileSync(file, "utf8");
  if (!html.includes("/assets/site.css")) errors.push(`Missing stylesheet link: ${file}`);
  const links = [...html.matchAll(/href="([^"]+)"/g)].map((match) => match[1]);
  for (const rawLink of links) {
    const link = rawLink.split("#")[0].split("?")[0];
    if (!link || /^(https?:|mailto:|#)/.test(link)) continue;
    if (link.startsWith("/assets/")) {
      if (!existsSync(join(root, link))) errors.push(`Broken asset link in ${file}: ${link}`);
      continue;
    }
    const target = normalize(join(root, link, "index.html"));
    if (!existsSync(target)) errors.push(`Broken route link in ${file}: ${link}`);
  }
}

if (errors.length > 0) {
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log(`Checked ${htmlFiles.length} HTML files and ${requiredRoutes.length} routes.`);
