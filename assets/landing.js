const replayButton = document.querySelector("[data-receipt-replay]");
const receiptRows = [...document.querySelectorAll(".hero-receipt .receipt-line")];

function replayReceipt() {
  receiptRows.forEach((row, index) => {
    row.classList.remove("stepped");
    row.style.setProperty("--step-index", index);
  });
  requestAnimationFrame(() => {
    receiptRows.forEach((row) => row.classList.add("stepped"));
  });
}

if (replayButton && receiptRows.length > 0) {
  replayButton.addEventListener("click", replayReceipt);
  replayReceipt();
}
