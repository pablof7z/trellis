export const FORMAT_VERSION = 4;

export function validateLabelRegistry(registry, errors) {
  if (!registry || typeof registry !== "object" || Array.isArray(registry)) {
    errors.push("labelRegistry: missing required object");
    return;
  }
  for (const [field, idField] of [
    ["nodes", "id"],
    ["scopes", "id"],
    ["resources", "key"],
    ["outputs", "key"],
  ]) {
    const entries = registry[field];
    if (!Array.isArray(entries)) {
      errors.push(`labelRegistry.${field}: missing required array`);
      continue;
    }
    entries.forEach((entry, index) => {
      if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
        errors.push(`labelRegistry.${field}[${index}]: expected an object`);
        return;
      }
      if (entry[idField] == null) errors.push(`labelRegistry.${field}[${index}].${idField}: missing required value`);
      if (typeof entry.label !== "string" || entry.label.length === 0) {
        errors.push(`labelRegistry.${field}[${index}].label: missing required string`);
      }
    });
  }
}

export function normalizeLabelRegistry(registry = {}) {
  return {
    nodes: labelMap(registry.nodes, "id"),
    scopes: labelMap(registry.scopes, "id"),
    resources: labelMap(registry.resources, "key"),
    outputs: labelMap(registry.outputs, "key"),
    raw: registry,
  };
}

export function nodeLabel(labels, id) {
  return labelFor(labels.nodes, id, "node");
}

export function scopeLabel(labels, id) {
  return labelFor(labels.scopes, id, "scope");
}

export function resourceLabel(labels, key) {
  return labelFor(labels.resources, key, "resource");
}

export function outputLabel(labels, key) {
  return labelFor(labels.outputs, key, "output");
}

export function labelSummary(registry) {
  const counts = [
    ["nodes", registry.nodes.size],
    ["scopes", registry.scopes.size],
    ["resources", registry.resources.size],
    ["outputs", registry.outputs.size],
  ];
  return counts.map(([name, count]) => `${count} ${name}`).join(" / ");
}

function labelMap(entries = [], keyField) {
  return new Map(entries.map((entry) => [String(entry[keyField]), entry.label]));
}

function labelFor(map, id, fallbackPrefix) {
  const key = String(id);
  return map.get(key) ?? `${fallbackPrefix}/${key}`;
}
