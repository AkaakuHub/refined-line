(() => {
  console.log("load hotkeys.js");
  if (window.__refinedLineHotkeysInstalled) return;
  window.__refinedLineHotkeysInstalled = true;

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
        console.error("[refined-line] hotkey getInvoke failed", error);
      }
      return null;
    }
    window.__refinedLineInvokeSource = "none";
    return null;
  };

  window.addEventListener(
    "keydown",
    (event) => {
      if (event.repeat) return;
      const isH = event.key.toLowerCase() === "h" || event.code === "KeyH";
      if (!event.altKey || !isH) return;
      const invoke = getInvoke();
      if (!invoke) {
        if (typeof console !== "undefined") {
          console.warn("[refined-line] hotkey invoke not found");
        }
        return;
      }
      invoke("toggle_content_protection").catch((error) => {
        if (typeof console !== "undefined") {
          console.error("[refined-line] hotkey invoke failed", error);
        }
      });
    },
    { capture: true },
  );
})();
