(() => {
  console.log("load no-scrollbar.js");
  const styleId = "refined-line-scrollbar-style";
  if (document.getElementById(styleId)) return;

  const style = document.createElement("style");
  style.id = styleId;
  style.textContent = `
    :root {
      --refined-line-scrollbar-size: 10px;
      --refined-line-scrollbar-thumb: rgba(128, 128, 128, 0.45);
      --refined-line-scrollbar-thumb-hover: rgba(128, 128, 128, 0.65);
      --refined-line-scrollbar-thumb-active: rgba(128, 128, 128, 0.8);
      scrollbar-gutter: auto;
    }

    ::-webkit-scrollbar {
      width: var(--refined-line-scrollbar-size);
      height: var(--refined-line-scrollbar-size);
    }

    ::-webkit-scrollbar-track {
      background: transparent;
    }

    ::-webkit-scrollbar-thumb {
      background: var(--refined-line-scrollbar-thumb);
      border-radius: 999px;
      border: 2px solid transparent;
      background-clip: padding-box;
    }

    ::-webkit-scrollbar-thumb:hover {
      background: var(--refined-line-scrollbar-thumb-hover);
    }

    ::-webkit-scrollbar-thumb:active {
      background: var(--refined-line-scrollbar-thumb-active);
    }

    ::-webkit-scrollbar-corner {
      background: transparent;
    }

    * {
      scrollbar-width: thin;
      scrollbar-color: var(--refined-line-scrollbar-thumb) transparent;
    }
  `;

  const injectStyle = () => {
    if (document.head && !document.head.contains(style)) {
      document.head.appendChild(style);
    }
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", injectStyle);
  } else {
    injectStyle();
  }

  const observer = new MutationObserver(() => {
    if (document.head && !document.head.contains(style)) {
      injectStyle();
    }
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
})();
