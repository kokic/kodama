
document.addEventListener("DOMContentLoaded", () => {
  document.title = [...document.querySelector("article h1").childNodes]
    .find(x => x.nodeType == Node.TEXT_NODE)
    .textContent
    .trim();
});
