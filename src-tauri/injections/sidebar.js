(() => {
  const wrap = document.querySelector(".pageLayout-module__wrap__h-oSt");
  if (!wrap) return;

  const splitCols = (s) => {
    const out = [];
    let cur = "";
    let depth = 0;
    for (const ch of s) {
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

  const current = () => getComputedStyle(wrap).gridTemplateColumns;
  const original = wrap.dataset.origCols || current();
  wrap.dataset.origCols = original;

  const collapsed = (() => {
    const cols = splitCols(original);
    if (cols.length < 3) return original;
    cols[1] = "0px";
    return cols.join(" ");
  })();

  const toggle = () => {
    const cur = current();
    wrap.style.gridTemplateColumns = cur === collapsed ? original : collapsed;
  };

  document.addEventListener("keydown", (e) => {
    if (e.altKey && e.key.toLowerCase() === "l") toggle();
  });
})();
