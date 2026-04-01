import { classifyUrl } from "../shared/engine.js";
import { logger } from "../../../npm/runtime/src/logger.js";

document.addEventListener("mouseup", async () => {
  const selection = window.getSelection()?.toString().trim();
  if (!selection) return;

  logger.info('Text selected', { 
    selection: selection.substring(0, 50) + (selection.length > 50 ? '...' : ''),
    url: window.location.href 
  });

  const payload = {
    surface: "ContentScript",
    action: "AnalyzeSelection",
    snapshot: {
      url: window.location.href,
      title: document.title,
      selected_text: selection
    }
  };

  try {
    const result = await chrome.runtime.sendMessage({
      type: "extro.command",
      payload
    });

    logger.info('Selection analysis result', { result });
    logger.debug('URL classification', { type: classifyUrl(window.location.href) });
  } catch (error) {
    logger.error('Failed to send selection', error);
  }
});

