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
  "/demos",
  "/demos/leak-duel",
  "/demos/flight-recorder",
  "/adr",
  "/examples",
  "/examples/local-first-sync",
  "/examples/language-server",
  "/examples/iot-dashboard",
  "/examples/plugin-runtime",
  "/examples/market-data",
  "/examples/nostr-feed",
  "/animations",
  "/animations/hidden-graph",
  "/animations/signal-vs-trellis",
  "/animations/shrinking-set",
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
    if ([".git", ".vercel", "dist", "node_modules", "target"].includes(entry)) return [];
    if (path.includes("examples/codebase-observatory")) return [];
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

function routePath(route) {
  return route === "/" ? join(root, "index.html") : join(root, route, "index.html");
}

const htmlFiles = walk(root).filter((path) => extname(path) === ".html");
const errors = [];
const flightRecorderTraces = [
  ["normal-session.json", { closeCommand: true }],
  ["seeded-leak.json", { failingInvariant: true, missingCloseOnRemovedDiff: true }],
  ["teardown-cascade.json", { closeCommand: true, childBeforeParentClose: true }]
];

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
    const fileTarget = normalize(join(root, link));
    if (existsSync(fileTarget) && statSync(fileTarget).isFile()) continue;
    const target = normalize(join(root, link, "index.html"));
    if (!existsSync(target)) errors.push(`Broken route link in ${file}: ${link}`);
  }
}

for (const [file, expectations] of flightRecorderTraces) {
  validateFlightRecorderTrace(file, expectations);
}

if (errors.length > 0) {
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log(`Checked ${htmlFiles.length} HTML files, ${requiredRoutes.length} routes, and ${flightRecorderTraces.length} trace fixtures.`);

function validateFlightRecorderTrace(file, expectations) {
  const tracePath = join(root, "demos", "flight-recorder", "traces", file);
  if (!existsSync(tracePath)) {
    errors.push(`Missing flight recorder trace: ${file}`);
    return;
  }

  let traceFile;
  try {
    traceFile = JSON.parse(readFileSync(tracePath, "utf8"));
  } catch (error) {
    errors.push(`Invalid JSON trace ${file}: ${error.message}`);
    return;
  }

  if (traceFile.formatVersion !== 1) {
    errors.push(`Unsupported trace format in ${file}: ${traceFile.formatVersion}`);
  }
  if (!Array.isArray(traceFile.steps) || traceFile.steps.length === 0) {
    errors.push(`Trace has no steps: ${file}`);
    return;
  }

  let hasCloseCommand = false;
  let hasFailingInvariant = false;
  let hasMissingCloseOnRemovedDiff = false;
  let hasChildBeforeParentClose = false;

  traceFile.steps.forEach((step, index) => {
    const prefix = `${file} step ${index + 1}`;
    const trace = step.trace ?? {};
    if (typeof step.name !== "string" || step.name.length === 0) {
      errors.push(`${prefix} is missing a name`);
    }
    if (!Number.isInteger(trace.transaction_id) || !Number.isInteger(trace.revision)) {
      errors.push(`${prefix} is missing transaction ids`);
    }
    for (const field of ["resource_commands", "collection_diffs", "phase_trace", "invariant_results"]) {
      if (!Array.isArray(trace[field])) errors.push(`${prefix} trace.${field} must be an array`);
    }

    const commands = Array.isArray(trace.resource_commands) ? trace.resource_commands : [];
    const diffs = Array.isArray(trace.collection_diffs) ? trace.collection_diffs : [];
    const invariants = Array.isArray(trace.invariant_results) ? trace.invariant_results : [];
    hasCloseCommand ||= commands.some((command) => command.kind === "Close");
    hasFailingInvariant ||= invariants.some((check) => check.passed === false);
    hasMissingCloseOnRemovedDiff ||= diffs.some((diff) => diff.removed > 0) && commands.length === 0;
    hasChildBeforeParentClose ||= commands
      .filter((command) => command.kind === "Close")
      .map((command) => command.scope)
      .join(",") === "9,9,8";

    for (const command of commands) {
      if (!["Open", "Close"].includes(command.kind) || typeof command.key !== "string" || !Number.isInteger(command.scope)) {
        errors.push(`${prefix} has invalid resource command`);
      }
    }
  });

  if (expectations.closeCommand && !hasCloseCommand) errors.push(`${file} must include a Close command`);
  if (expectations.failingInvariant && !hasFailingInvariant) errors.push(`${file} must include a failing invariant`);
  if (expectations.missingCloseOnRemovedDiff && !hasMissingCloseOnRemovedDiff) {
    errors.push(`${file} must show a removed diff without a close command`);
  }
  if (expectations.childBeforeParentClose && !hasChildBeforeParentClose) {
    errors.push(`${file} must close child scope resources before parent scope resources`);
  }
}
