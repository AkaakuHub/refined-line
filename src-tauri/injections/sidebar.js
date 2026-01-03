(() => {
  const splitCols = (value) => {
    const out = [];
    let cur = "";
    let depth = 0;
    for (const ch of value) {
      if (ch === "(") depth += 1;
      if (ch === ")") depth -= 1;
      if (ch === " " && depth === 0) {
        if (cur) out.push(cur);
        cur = "";
        continue;
      }
      cur += ch;
    }
    if (cur) out.push(cur);
    return out;
  };

  const findWrap = () =>
    document.querySelector(".pageLayout-module__wrap__h-oSt");
  const current = (node) => getComputedStyle(node).gridTemplateColumns;

  let originalCols = null;
  let collapsedCols = null;
  let isCollapsed = false;

  const buildCollapsed = (value) => {
    const cols = splitCols(value);
    if (cols.length >= 2) {
      cols[1] = "0px";
    }
    if (cols.length >= 3) {
      cols[2] = "minmax(0, auto)";
    }
    return cols.join(" ");
  };

  const ensureTemplates = (node) => {
    if (!originalCols) {
      originalCols = current(node);
      collapsedCols = buildCollapsed(originalCols);
    }
  };

  const toggle = () => {
    const wrap = findWrap();
    if (!wrap) return;
    ensureTemplates(wrap);
    wrap.style.gridTemplateColumns = isCollapsed ? originalCols : collapsedCols;
    isCollapsed = !isCollapsed;
  };

  window.addEventListener(
    "keydown",
    (e) => {
      const isL = e.key.toLowerCase() === "l" || e.code === "KeyL";
      if (e.altKey && isL) toggle();
    },
    { capture: true },
  );
})();
