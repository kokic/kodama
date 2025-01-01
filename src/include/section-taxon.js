
document.addEventListener("DOMContentLoaded", () => {
  document.querySelector("#toc").querySelectorAll("a:not(.bullet)").forEach(e => {
    const targetId = e.href.substring(e.href.lastIndexOf("#"));
    const taxon = e.querySelector("span.taxon").innerHTML;
    document.querySelector("article")
      .querySelector(targetId)
      .querySelector("span.taxon")
      .innerHTML = taxon;
  })
});
