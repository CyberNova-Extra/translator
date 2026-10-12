"use strict";

const HOST = "qwen_translate";
let port = null;
let queue = [];
let busy = false;

function notifyError(tabId, message) {
  browser.tabs
    .sendMessage(tabId, { type: "qt-result", error: message })
    .catch(() => {});
}

function ensurePort() {
  if (port) return true;
  try {
    port = browser.runtime.connectNative(HOST);
  } catch (e) {
    return false;
  }
  port.onMessage.addListener((msg) => {
    const req = queue.shift();
    busy = false;
    if (req) {
      browser.tabs
        .sendMessage(req.tabId, { type: "qt-result", ...msg })
        .catch(() => {});
    }
    pump();
  });
  port.onDisconnect.addListener(() => {
    port = null;
    busy = false;
    const err = browser.runtime.lastError;
    const message = err && err.message ? err.message : "翻译程序连接断开";
    const pending = queue;
    queue = [];
    for (const p of pending) notifyError(p.tabId, message);
  });
  return true;
}

function pump() {
  if (busy || queue.length === 0) return;
  if (!ensurePort()) {
    const pending = queue;
    queue = [];
    for (const p of pending) {
      notifyError(p.tabId, "无法启动翻译程序（native host 未安装？）");
    }
    return;
  }
  busy = true;
  port.postMessage({ text: queue[0].text });
}

browser.contextMenus.create({
  id: "qt-translate-selection",
  title: "翻译选中内容",
  contexts: ["selection"],
});

browser.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== "qt-translate-selection" || !tab || tab.id == null) return;
  const text = (info.selectionText || "").trim();
  if (!text) return;
  browser.tabs.sendMessage(tab.id, { type: "qt-show", text }).catch(() => {});
  queue.push({ tabId: tab.id, text });
  pump();
});
