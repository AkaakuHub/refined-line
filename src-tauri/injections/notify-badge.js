(() => {
  console.log("load notify-badge.js");
  if (window.__refinedLineNotifyBadgeInstalled) return;
  window.__refinedLineNotifyBadgeInstalled = true;

  const getInvoke = () => {
    try {
      const tauri = window.__TAURI__;
      if (tauri?.core?.invoke) {
        window.__refinedLineInvokeSource = "__TAURI__.core.invoke";
        return tauri.core.invoke;
      }
    } catch (error) {
      window.__refinedLineInvokeSource = "error";
      if (typeof console !== "undefined") {
        console.error("[refined-line] notify badge getInvoke failed", error);
      }
      return null;
    }
    window.__refinedLineInvokeSource = "none";
    return null;
  };

  const sendBadgeText = (text) => {
    const invoke = getInvoke();
    if (!invoke) return;
    invoke("update_notification_badge", { text: text === undefined ? null : String(text) }).catch(() => {});
  };

  const patchChromeAction = () => {
    const action = window.chrome?.action;
    if (!action || typeof action.setBadgeText !== "function") {
      return false;
    }
    if (action.__refinedLineSetBadgeTextPatched) {
      return true;
    }

    const originalSetBadgeText = action.setBadgeText.bind(action);
    action.setBadgeText = (details) => {
      const text = details && typeof details === "object" ? details.text : null;
      sendBadgeText(text);
      return originalSetBadgeText(details);
    };
    action.__refinedLineSetBadgeTextPatched = true;

    if (typeof action.getBadgeText === "function") {
      action.getBadgeText({}).then((text) => sendBadgeText(text)).catch(() => {});
    }

    return true;
  };

  if (patchChromeAction()) {
    return;
  }
})();
