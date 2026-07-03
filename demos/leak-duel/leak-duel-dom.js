export function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function fact(label, value, className = "fact-row") {
  const node = el("div", className);
  node.append(el("strong", "", label), el("span", "", value));
  return node;
}

export function badge(text, tone = "") {
  return el("span", `bracket-badge ${tone}`.trim(), text);
}

export function setText(selector, value) {
  const node = document.querySelector(selector);
  if (node) node.textContent = String(value);
}

export function signed(value) {
  return value > 0 ? `+${value}` : String(value);
}

export function shortKey(key) {
  return String(key).replace(/^attachment:/, "").replaceAll(":", " / ");
}

export function commandKind(kind) {
  return `[${String(kind || "none").toUpperCase()}]`;
}

export function clearTimer(timer) {
  if (timer) clearInterval(timer);
  return null;
}
