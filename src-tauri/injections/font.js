(() => {
  console.log("load font.js");
  const id = "refined-line-font-override";
  if (document.getElementById(id)) return;
  const style = document.createElement("style");
  style.id = id;
  style.textContent = `
    :root, body, * {
      font-family: ui-rounded!important;
    }
  `;
  document.documentElement.appendChild(style);
})();
