(() => {
  console.log("load sidebar.js");
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
  const findParent = () =>
    document.querySelector(".chatroomContent-module__content_area__gK6db");
  const current = (node) => getComputedStyle(node).gridTemplateColumns;

  let originalCols = null;
  let collapsedCols = null;
  let isCollapsed = false;
  let toggleEl = null;
  let chevron = null;

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

  const setChevronDirection = (direction) => {
    if (!chevron) return;
    const { lineA, lineB } = chevron;
    if (direction === "left") {
      lineA.style.transform = "rotate(-45deg)";
      lineB.style.transform = "rotate(45deg)";
    } else {
      lineA.style.transform = "rotate(45deg)";
      lineB.style.transform = "rotate(-45deg)";
    }
  };

  const buildChevron = () => {
    const icon = document.createElement("div");
    icon.style.width = "10px";
    icon.style.height = "10px";
    icon.style.position = "relative";

    const lineA = document.createElement("div");
    lineA.style.position = "absolute";
    lineA.style.top = "2px";
    lineA.style.left = "2px";
    lineA.style.width = "6px";
    lineA.style.height = "1px";
    lineA.style.background = "#202a43";
    lineA.style.borderRadius = "2px";
    lineA.style.transformOrigin = "center";

    const lineB = document.createElement("div");
    lineB.style.position = "absolute";
    lineB.style.top = "6px";
    lineB.style.left = "2px";
    lineB.style.width = "6px";
    lineB.style.height = "1px";
    lineB.style.background = "#202a43";
    lineB.style.borderRadius = "2px";
    lineB.style.transformOrigin = "center";

    icon.appendChild(lineA);
    icon.appendChild(lineB);

    chevron = { icon, lineA, lineB };
    return chevron;
  };

  const ensureUi = () => {
    const parent = findParent();
    if (!parent) return null;

    if (getComputedStyle(parent).position === "static") {
      parent.style.position = "relative";
    }

    if (!toggleEl || !parent.contains(toggleEl)) {
      if (toggleEl) toggleEl.remove();
      const el = document.createElement("div");
      el.id = "refined-line-sidebar-toggle";
      el.setAttribute("role", "button");
      el.setAttribute("aria-label", "Toggle sidebar");
      el.title = "サイドバーの表示を切り替え (Alt+L)";
      el.style.position = "absolute";
      el.style.top = "16px";
      el.style.left = "0";
      el.style.width = "12px";
      el.style.height = "40px";
      el.style.borderRadius = "0 4px 4px 0";
      el.style.background = "#fff";
      el.style.border = "1px solid #ddd";
      el.style.borderLeft = "none";
      el.style.cursor = "pointer";
      el.style.zIndex = "9999";
      el.style.display = "flex";
      el.style.alignItems = "center";
      el.style.justifyContent = "center";
      el.addEventListener("click", () => toggle());
      const icon = buildChevron();
      el.appendChild(icon.icon);
      parent.appendChild(el);
      toggleEl = el;
    }

    return toggleEl;
  };

  const updateUi = () => {
    const el = ensureUi();
    if (!el) return;
    setChevronDirection(isCollapsed ? "right" : "left");
  };

  const toggle = () => {
    const wrap = findWrap();
    if (!wrap) return;
    ensureTemplates(wrap);
    wrap.style.gridTemplateColumns = isCollapsed ? originalCols : collapsedCols;
    isCollapsed = !isCollapsed;
    updateUi();
  };

  const onDomMutated = () => {
    if (ensureUi()) {
      updateUi();
    }
  };

  onDomMutated();
  const observer = new MutationObserver(onDomMutated);
  observer.observe(document.documentElement, {
    childList: true,
    subtree: true,
  });

  window.addEventListener(
    "keydown",
    (e) => {
      const isL = e.key.toLowerCase() === "l" || e.code === "KeyL";
      if (e.altKey && isL) toggle();
    },
    { capture: true },
  );
})();
