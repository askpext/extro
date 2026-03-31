import { runCore } from "../shared/engine.js";
import { logger } from "../shared/logger.js";

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type !== "extro.command") return false;

  logger.command(message.payload, sender.id || 'unknown');

  runCore(message.payload)
    .then((result) => {
      logger.info('Command executed successfully', { result, sender });
      
      for (const effect of result.effects) {
        applyEffect(effect, sender);
      }
      sendResponse(result);
    })
    .catch((error) => {
      logger.error('Command execution failed', error);
      sendResponse({ error: String(error) });
    });

  return true;
});

async function applyEffect(effect, sender) {
  logger.debug('Applying effect', { effect, sender });
  
  switch (true) {
    case "PersistSession" in effect:
      await chrome.storage.session.set({
        [effect.PersistSession.key]: effect.PersistSession.value,
      });
      logger.info('Session persisted', { key: effect.PersistSession.key });
      break;
    case "ShowPopupToast" in effect:
      await chrome.runtime.sendMessage({
        type: "extro.toast",
        payload: effect.ShowPopupToast,
      });
      logger.info('Toast shown', effect.ShowPopupToast);
      break;
    case "OpenSidePanel" in effect:
      if (chrome.sidePanel && sender.tab?.windowId) {
        await chrome.sidePanel.open({ windowId: sender.tab.windowId });
        logger.info('Side panel opened', { route: effect.OpenSidePanel.route });
      }
      break;
    default:
      logger.warn('Unknown effect type', { effect });
      break;
  }
}

