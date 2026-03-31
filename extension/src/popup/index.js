document.querySelector("#summarize").addEventListener("click", async () => {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  const output = document.querySelector("#output");

  const result = await chrome.runtime.sendMessage({
    type: "extro.command",
    payload: {
      surface: "Popup",
      action: "SummarizePage",
      snapshot: {
        url: tab.url,
        title: tab.title,
        selected_text: null
      }
    }
  });

  output.textContent = JSON.stringify(result, null, 2);
});

