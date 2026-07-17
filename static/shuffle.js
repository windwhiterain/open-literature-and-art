(function() {
  const list = document.getElementById("card-list");
  const btn = document.getElementById("shuffle-btn");
  if (!list) return;

  function shuffle() {
    const cards = Array.from(list.children);
    for (let i = cards.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      list.appendChild(cards[j]);
    }
  }

  shuffle();
  if (btn) btn.addEventListener("click", shuffle);
})();
