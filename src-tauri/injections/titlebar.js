(() => {
  console.log("load titlebar.js");
  // Avoid injecting duplicate titlebar
  if (document.getElementById("refined-line-titlebar-host")) return;

  const TITLEBAR_HEIGHT = 24;

  const host = document.createElement("div");
  host.id = "refined-line-titlebar-host";
  host.setAttribute("role", "presentation");
  host.style.position = "fixed";
  host.style.top = "0";
  host.style.left = "0";
  host.style.right = "0";
  host.style.height = `${TITLEBAR_HEIGHT}px`;
  host.style.zIndex = "2147483647";
  host.style.pointerEvents = "none";

  const shadow = host.attachShadow({ mode: "open" });

  const style = document.createElement("style");
  style.textContent = `
    :host {
      all: initial;
      position: fixed;
      inset: 0 0 auto 0;
      z-index: 2147483647;
      pointer-events: none;
    }

    *, *::before, *::after { box-sizing: border-box; }

    .bar {
      height: ${TITLEBAR_HEIGHT}px;
      display: flex;
      align-items: stretch;
      justify-content: flex-start;
      background: transparent;
      color: #cccccc;
      pointer-events: none;
      user-select: none;
      -webkit-font-smoothing: antialiased;
      font-family: "Segoe UI", "Yu Gothic UI", "Noto Sans JP", system-ui, sans-serif;
      font-size: 12px;
      line-height: 1;
    }

    .left { display: none; }

    .settings-button {
      appearance: none;
      border: none;
      background: transparent;
      color: #707991;
      width: 16px;
      height: 16px;
      padding: 0;
      margin: 0;
      margin-left: 6px;
      margin-top: 1px;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border-radius: 999px;
    }

    .settings-button svg {
      width: 16px;
      height: 16px;
      display: block;
    }

    .dropdown {
      position: absolute;
      top: ${TITLEBAR_HEIGHT}px;
      left: 0;
      min-width: 280px;
      max-width: 420px;
      background: #ffffff;
      color: #000000;
      border: 1px solid #c8c8c8;
      border-radius: 5px;
      padding: 6px 0;
      display: none;
      z-index: 2147483647;
      white-space: nowrap;
      overflow: hidden;
      box-shadow: 0 0 10px 0 var(--action-popover-popover-wrap-box-shadow);
    }

    .dropdown[data-open="true"] { display: block; }

    .item {
      width: 100%;
      border: none;
      background: transparent;
      color: inherit;
      display: grid;
      grid-template-columns: 18px minmax(0, 1fr) max-content;
      align-items: center;
      gap: 10px;
      padding: 7px 12px;
      cursor: pointer;
      text-align: left;
      font-size: 12px;
      white-space: nowrap;
      line-height: 1.2;
    }

    .item:hover { background: #fafafa; }
    .item:active { background: #fafafa; }

    .check {
      width: 16px;
      height: 16px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex: 0 0 auto;
    }

    .check-mark {
      width: 14px;
      height: 14px;
      border-radius: 3px;
      border: 1px solid #202a43;
      background: transparent;
      position: relative;
    }

    .item.is-radio .check-mark {
      border-radius: 999px;
    }

    .item.is-checked .check-mark {
      border-color: #202a43;
      background: #07b53b;
    }

    .item.is-checked.is-radio .check-mark {
      background: transparent;
      border-color: #202a43;
    }

    .item.is-checked.is-radio .check-mark::after {
      content: "";
      position: absolute;
      inset: 2px;
      border-radius: 999px;
      background: #07b53b;
    }

    .label {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      min-width: 0;
    }

    .shortcut {
      font-size: 11px;
      white-space: nowrap;
      padding-left: 14px;
    }

    .sep {
      height: 1px;
      margin: 6px 0;
      background: #efefef;
    }

    .right {
      display: flex;
      align-items: center;
      flex: 0 0 auto;
      position: relative;
      -webkit-app-region: no-drag;
      app-region: no-drag;
      pointer-events: auto;
      gap: 6px;
      margin-left: 7px;
    }

    .control-button {
      appearance: none;
      border: none;
      margin: 0;
      padding: 0;
      padding-left: 1.33px;
      width: 12px;
      height: 12px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border-radius: 999px;
      cursor: pointer;
      position: relative;
      color: rgba(0, 0, 0, 0.6);
    }

    .control-button .control-icon {
      width: 7px;
      height: 7px;
      opacity: 0;
      transition: opacity 0.12s ease;
    }

    .control-button:hover .control-icon,
    .control-button:focus-visible .control-icon {
      opacity: 1;
    }

    .control-button.minimize {
      background: #f6c343;
    }

    .control-button.maximize {
      background: #58c966;
    }

    .control-button.close {
      background: #f26a6a;
    }

    .control-button:hover {
      filter: brightness(1.05);
    }
  `;
  shadow.appendChild(style);

  const bar = document.createElement("div");
  bar.className = "bar";

  const menuButton = document.createElement("button");
  menuButton.className = "settings-button";
  menuButton.type = "button";
  menuButton.setAttribute("aria-label", "設定");
  menuButton.innerHTML =
    "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' aria-hidden='true' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'>" +
    "<path d='M19.875 6.27a2.225 2.225 0 0 1 1.125 1.948v7.284c0 .809 -.443 1.555 -1.158 1.948l-6.75 4.27a2.269 2.269 0 0 1 -2.184 0l-6.75 -4.27a2.225 2.225 0 0 1 -1.158 -1.948v-7.285c0 -.809 .443 -1.554 1.158 -1.947l6.75 -3.98a2.33 2.33 0 0 1 2.25 0l6.75 3.98h-.033z'/>" +
    "<path d='M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0'/>" +
    "</svg>";

  const dropdown = document.createElement("div");
  dropdown.className = "dropdown";
  dropdown.setAttribute("data-open", "false");

  const right = document.createElement("div");
  right.className = "right";

  const MENU_IDS = {
    contentProtection: "menu.content_protection",
    autostart: "menu.autostart",
    startMinimized: "menu.start_minimized",
    logError: "menu.log.error",
    logWarn: "menu.log.warn",
    logInfo: "menu.log.info",
    logDebug: "menu.log.debug",
    logVerbose: "menu.log.verbose"
  };

  const menuItemElements = new Map();

  const getTauriInvoke = () => window.__TAURI__?.core?.invoke;
  const getCurrentWindow = () => window.__TAURI__?.window?.getCurrentWindow?.();
  const getIsMaximized = async () => {
    const invoke = getTauriInvoke();
    if (!invoke) throw new Error("Tauri invoke not available");
    return await invoke("get_is_maximized");
  };
  const getIsDev = async () => {
    const invoke = getTauriInvoke();
    if (!invoke) return false;
    try {
      return await invoke("get_is_dev");
    } catch (error) {
      console.warn("[menu] get_is_dev failed", error);
      return false;
    }
  };

  const baseMenuItems = [
    {
      id: MENU_IDS.contentProtection,
      label: "画面を保護",
      type: "check",
      shortcut: "Alt+H"
    },
    {
      id: MENU_IDS.autostart,
      label: "Windows 起動時に自動起動",
      type: "check"
    },
    {
      id: MENU_IDS.startMinimized,
      label: "起動時に最小化",
      type: "check"
    }
  ];

  const logMenuItems = [
    {
      id: MENU_IDS.logError,
      label: "ログレベル: Error",
      type: "radio",
      value: "error"
    },
    {
      id: MENU_IDS.logWarn,
      label: "ログレベル: Warn",
      type: "radio",
      value: "warn"
    },
    {
      id: MENU_IDS.logInfo,
      label: "ログレベル: Info",
      type: "radio",
      value: "info"
    },
    {
      id: MENU_IDS.logDebug,
      label: "ログレベル: Debug",
      type: "radio",
      value: "debug"
    },
    {
      id: MENU_IDS.logVerbose,
      label: "ログレベル: Verbose",
      type: "radio",
      value: "verbose"
    }
  ];

  const buildMenuModel = (isDev) => {
    const items = [...baseMenuItems, { type: "separator" }];
    if (isDev) {
      items.push(...logMenuItems, { type: "separator" });
    }
    items.push({
      id: "window.close",
      label: "閉じる",
      type: "action"
    });
    return items;
  };

  const setMenuOpen = (open) => {
    dropdown.setAttribute("data-open", open ? "true" : "false");
    menuButton.classList.toggle("is-open", open);
  };

  const buildMenuItem = (item) => {
    if (item.type === "separator") {
      const sep = document.createElement("div");
      sep.className = "sep";
      dropdown.appendChild(sep);
      return;
    }

    const button = document.createElement("button");
    button.type = "button";
    button.className = "item";
    button.dataset.menuId = item.id;
    button.dataset.menuType = item.type;

    if (item.type === "radio") {
      button.classList.add("is-radio");
    }

    let check = null;
    if (item.type !== "action") {
      check = document.createElement("span");
      check.className = "check";

      const checkMark = document.createElement("span");
      checkMark.className = "check-mark";
      check.appendChild(checkMark);
    } else {
      check = document.createElement("span");
    }

    const label = document.createElement("span");
    label.className = "label";
    label.textContent = item.label;

    const shortcut = document.createElement("span");
    shortcut.className = "shortcut";
    shortcut.textContent = item.shortcut || "";

    button.appendChild(check);
    button.appendChild(label);
    button.appendChild(shortcut);

    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      setMenuOpen(false);

      if (item.type === "action") {
        if (item.id === "window.close") {
          const currentWindow = getCurrentWindow();
          if (currentWindow) {
            await currentWindow.close();
          }
        }
        return;
      }

      const invoke = getTauriInvoke();
      if (!invoke) return;

      try {
        await invoke("menu_action", { id: item.id });
      } catch (error) {
        console.warn("[menu] action failed", error);
      }

      await refreshMenuState();
    });

    dropdown.appendChild(button);
    menuItemElements.set(item.id, button);
  };

  const renderMenu = (items) => {
    dropdown.innerHTML = "";
    menuItemElements.clear();
    items.forEach(buildMenuItem);
  };

  renderMenu(buildMenuModel(false));

  const setMenuItemChecked = (id, checked) => {
    const button = menuItemElements.get(id);
    if (!button) return;
    button.classList.toggle("is-checked", checked);
  };

  const setLogLevelChecked = (level) => {
    const logItems = [
      { id: MENU_IDS.logError, value: "error" },
      { id: MENU_IDS.logWarn, value: "warn" },
      { id: MENU_IDS.logInfo, value: "info" },
      { id: MENU_IDS.logDebug, value: "debug" },
      { id: MENU_IDS.logVerbose, value: "verbose" }
    ];
    logItems.forEach(({ id, value }) => {
      setMenuItemChecked(id, value === level);
    });
  };

  const refreshMenuState = async () => {
    const invoke = getTauriInvoke();
    if (!invoke) return;

    try {
      const [settings, protectedState] = await Promise.all([
        invoke("get_settings"),
        invoke("get_content_protection")
      ]);

      setMenuItemChecked(MENU_IDS.contentProtection, !!protectedState);
      setMenuItemChecked(MENU_IDS.autostart, !!settings?.autoStart);
      setMenuItemChecked(MENU_IDS.startMinimized, !!settings?.startMinimized);
      setLogLevelChecked(settings?.logLevel || "info");
    } catch (error) {
      console.warn("[menu] refresh failed", error);
    }
  };

  menuButton.addEventListener("click", (event) => {
    event.stopPropagation();
    const isOpen = dropdown.getAttribute("data-open") === "true";
    setMenuOpen(!isOpen);
  });

  const isEventInsideHost = (event) => {
    const path = event.composedPath ? event.composedPath() : [];
    return path.includes(host);
  };

  document.addEventListener("click", (event) => {
    if (!isEventInsideHost(event)) {
      setMenuOpen(false);
      return;
    }
    const path = event.composedPath ? event.composedPath() : [];
    const clickedMenuArea = path.includes(menuButton) || path.includes(dropdown) || path.includes(right);
    if (!clickedMenuArea) {
      setMenuOpen(false);
    }
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      setMenuOpen(false);
    }
  });

  const maybeListenContentProtection = () => {
    const listen = window.__TAURI__?.event?.listen;
    if (!listen) return;
    listen("content-protection-changed", (event) => {
      setMenuItemChecked(MENU_IDS.contentProtection, !!event.payload);
    });
  };

  const syncMenuForDev = async () => {
    const isDev = await getIsDev();
    if (!isDev) return;
    renderMenu(buildMenuModel(true));
    await refreshMenuState();
  };

  const minimizeBtn = document.createElement("button");
  minimizeBtn.className = "control-button minimize";
  minimizeBtn.type = "button";
  minimizeBtn.title = "Minimize";
  minimizeBtn.innerHTML =
    "<svg class='control-icon' xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12' aria-hidden='true'>" +
    "<line x1='2' y1='6' x2='10' y2='6' stroke='currentColor' stroke-width='2' stroke-linecap='round'/>" +
    "</svg>";
  minimizeBtn.addEventListener("click", () => {
    const currentWindow = getCurrentWindow();
    if (currentWindow) {
      currentWindow.minimize();
    }
  });

  const maximizeBtn = document.createElement("button");
  maximizeBtn.className = "control-button maximize";
  maximizeBtn.type = "button";
  const maximizeIcon =
    "<svg class='control-icon' xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12' aria-hidden='true'>" +
    "<line x1='2' y1='6' x2='10' y2='6' stroke='currentColor' stroke-width='2' stroke-linecap='round'/>" +
    "<line x1='6' y1='2' x2='6' y2='10' stroke='currentColor' stroke-width='2' stroke-linecap='round'/>" +
    "</svg>";

  const restoreIcon =
    "<svg class='control-icon' xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12' aria-hidden='true' style='scale(1.5)'>" +
    "<circle cx='6' cy='6' r='3' stroke='currentColor' stroke-width='1' fill='currentColor'/>" +
    "</svg>";

  const setMaximizeIcon = (isMaximized) => {
    maximizeBtn.innerHTML = isMaximized ? restoreIcon : maximizeIcon;
    maximizeBtn.title = isMaximized ? "Restore" : "Maximize";
  };

  const refreshMaximizeState = async () => {
    const isMaximized = await getIsMaximized();
    setMaximizeIcon(isMaximized);
  };

  maximizeBtn.addEventListener("click", async () => {
    const currentWindow = getCurrentWindow();
    if (!currentWindow) return;

    const wasMaximized = await getIsMaximized();
    await currentWindow.toggleMaximize();
    setMaximizeIcon(!wasMaximized);
  });

  const closeBtn = document.createElement("button");
  closeBtn.className = "control-button close";
  closeBtn.type = "button";
  closeBtn.title = "Close";
  closeBtn.innerHTML =
    "<svg class='control-icon' xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12' aria-hidden='true'>" +
    "<line x1='2' y1='2' x2='10' y2='10' stroke='currentColor' stroke-width='2' stroke-linecap='round'/>" +
    "<line x1='10' y1='2' x2='2' y2='10' stroke='currentColor' stroke-width='2' stroke-linecap='round'/>" +
    "</svg>";
  closeBtn.addEventListener("click", () => {
    const currentWindow = getCurrentWindow();
    if (currentWindow) {
      currentWindow.close();
    }
  });

  right.appendChild(closeBtn);
  right.appendChild(minimizeBtn);
  right.appendChild(maximizeBtn);
  right.appendChild(menuButton);
  right.appendChild(dropdown);

  const bindWindowStateSync = () => {
    const currentWindow = getCurrentWindow();
    if (currentWindow?.onResized) {
      currentWindow.onResized(refreshMaximizeState).catch(() => {});
    } else {
      const listen = window.__TAURI__?.event?.listen;
      if (listen) {
        listen("tauri://resize", refreshMaximizeState);
      }
    }
  };

  bar.appendChild(right);
  shadow.appendChild(bar);

  const ensureGlobalStyle = () => {
    const globalId = "refined-line-titlebar-global-style";
    if (document.getElementById(globalId)) return;
    const globalStyle = document.createElement("style");
    globalStyle.id = globalId;
    globalStyle.textContent = `
      body {
        -webkit-app-region: drag;
        app-region: drag;
      }
      a, button, input, textarea, select, option, [role="button"], [role="link"],
      [contenteditable="true"], [contenteditable=""], [contenteditable="plaintext-only"],
      .chatroom-module__chatroom__eVUaK
      {
        -webkit-app-region: no-drag;
        app-region: no-drag;
      }
    `;
    if (document.head) {
      document.head.appendChild(globalStyle);
    } else {
      document.documentElement.appendChild(globalStyle);
    }
  };

  const injectTitlebar = () => {
    if (document.body && !document.body.contains(host)) {
      document.body.insertBefore(host, document.body.firstChild);
    }
    if (document.body) {
      document.body.setAttribute("data-tauri-drag-region", "");
    }
    ensureGlobalStyle();
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", injectTitlebar);
  } else {
    injectTitlebar();
  }

  refreshMenuState();
  maybeListenContentProtection();
  syncMenuForDev();
  refreshMaximizeState();
  bindWindowStateSync();

  const observer = new MutationObserver(() => {
    if (document.body && !document.body.contains(host)) {
      injectTitlebar();
    }
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
})();
