true              &&(function polyfill() {
  const relList = document.createElement("link").relList;
  if (relList && relList.supports && relList.supports("modulepreload")) {
    return;
  }
  for (const link of document.querySelectorAll('link[rel="modulepreload"]')) {
    processPreload(link);
  }
  new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type !== "childList") {
        continue;
      }
      for (const node of mutation.addedNodes) {
        if (node.tagName === "LINK" && node.rel === "modulepreload")
          processPreload(node);
      }
    }
  }).observe(document, { childList: true, subtree: true });
  function getFetchOpts(link) {
    const fetchOpts = {};
    if (link.integrity) fetchOpts.integrity = link.integrity;
    if (link.referrerPolicy) fetchOpts.referrerPolicy = link.referrerPolicy;
    if (link.crossOrigin === "use-credentials")
      fetchOpts.credentials = "include";
    else if (link.crossOrigin === "anonymous") fetchOpts.credentials = "omit";
    else fetchOpts.credentials = "same-origin";
    return fetchOpts;
  }
  function processPreload(link) {
    if (link.ep)
      return;
    link.ep = true;
    const fetchOpts = getFetchOpts(link);
    fetch(link.href, fetchOpts);
  }
}());

/******************************************************************************
Copyright (c) Microsoft Corporation.

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
***************************************************************************** */
/* global Reflect, Promise, SuppressedError, Symbol, Iterator */


typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
};

/**
 * Stores the callback in a known location, and returns an identifier that can be passed to the backend.
 * The backend uses the identifier to `eval()` the callback.
 *
 * @return An unique identifier associated with the callback function.
 *
 * @since 1.0.0
 */
function transformCallback(
// TODO: Make this not optional in v3
callback, once = false) {
    return window.__TAURI_INTERNALS__.transformCallback(callback, once);
}
/**
 * Sends a message to the backend.
 * @example
 * ```typescript
 * import { invoke } from '@tauri-apps/api/core';
 * await invoke('login', { user: 'tauri', password: 'poiwe3h4r5ip3yrhtew9ty' });
 * ```
 *
 * @param cmd The command name.
 * @param args The optional arguments to pass to the command.
 * @param options The request options.
 * @return A promise resolving or rejecting to the backend response.
 *
 * @since 1.0.0
 */
async function invoke(cmd, args = {}, options) {
    return window.__TAURI_INTERNALS__.invoke(cmd, args, options);
}

// Copyright 2019-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT
/**
 * The event system allows you to emit events to the backend and listen to events from it.
 *
 * This package is also accessible with `window.__TAURI__.event` when [`app.withGlobalTauri`](https://v2.tauri.app/reference/config/#withglobaltauri) in `tauri.conf.json` is set to `true`.
 * @module
 */
/**
 * @since 1.1.0
 */
var TauriEvent;
(function (TauriEvent) {
    TauriEvent["WINDOW_RESIZED"] = "tauri://resize";
    TauriEvent["WINDOW_MOVED"] = "tauri://move";
    TauriEvent["WINDOW_CLOSE_REQUESTED"] = "tauri://close-requested";
    TauriEvent["WINDOW_DESTROYED"] = "tauri://destroyed";
    TauriEvent["WINDOW_FOCUS"] = "tauri://focus";
    TauriEvent["WINDOW_BLUR"] = "tauri://blur";
    TauriEvent["WINDOW_SCALE_FACTOR_CHANGED"] = "tauri://scale-change";
    TauriEvent["WINDOW_THEME_CHANGED"] = "tauri://theme-changed";
    TauriEvent["WINDOW_CREATED"] = "tauri://window-created";
    TauriEvent["WEBVIEW_CREATED"] = "tauri://webview-created";
    TauriEvent["DRAG_ENTER"] = "tauri://drag-enter";
    TauriEvent["DRAG_OVER"] = "tauri://drag-over";
    TauriEvent["DRAG_DROP"] = "tauri://drag-drop";
    TauriEvent["DRAG_LEAVE"] = "tauri://drag-leave";
})(TauriEvent || (TauriEvent = {}));
/**
 * Unregister the event listener associated with the given name and id.
 *
 * @ignore
 * @param event The event name
 * @param eventId Event identifier
 * @returns
 */
async function _unlisten(event, eventId) {
    window.__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener(event, eventId);
    await invoke('plugin:event|unlisten', {
        event,
        eventId
    });
}
/**
 * Listen to an emitted event to any {@link EventTarget|target}.
 *
 * @example
 * ```typescript
 * import { listen } from '@tauri-apps/api/event';
 * const unlisten = await listen<string>('error', (event) => {
 *   console.log(`Got error, payload: ${event.payload}`);
 * });
 *
 * // you need to call unlisten if your handler goes out of scope e.g. the component is unmounted
 * unlisten();
 * ```
 *
 * @param event Event name. Must include only alphanumeric characters, `-`, `/`, `:` and `_`.
 * @param handler Event handler callback.
 * @param options Event listening options.
 * @returns A promise resolving to a function to unlisten to the event.
 * Note that removing the listener is required if your listener goes out of scope e.g. the component is unmounted.
 *
 * @since 1.0.0
 */
async function listen(event, handler, options) {
    var _a;
    const target = ((_a = void 0 ) !== null && _a !== void 0 ? _a : { kind: 'Any' });
    return invoke('plugin:event|listen', {
        event,
        target,
        handler: transformCallback(handler)
    }).then((eventId) => {
        return async () => _unlisten(event, eventId);
    });
}

/**
 * Open a file/directory selection dialog.
 *
 * The selected paths are added to the filesystem and asset protocol scopes.
 * When security is more important than the easy of use of this API,
 * prefer writing a dedicated command instead.
 *
 * Note that the scope change is not persisted, so the values are cleared when the application is restarted.
 * You can save it to the filesystem using [tauri-plugin-persisted-scope](https://github.com/tauri-apps/tauri-plugin-persisted-scope).
 * @example
 * ```typescript
 * import { open } from '@tauri-apps/plugin-dialog';
 * // Open a selection dialog for image files
 * const selected = await open({
 *   multiple: true,
 *   filters: [{
 *     name: 'Image',
 *     extensions: ['png', 'jpeg']
 *   }]
 * });
 * if (Array.isArray(selected)) {
 *   // user selected multiple files
 * } else if (selected === null) {
 *   // user cancelled the selection
 * } else {
 *   // user selected a single file
 * }
 * ```
 *
 * @example
 * ```typescript
 * import { open } from '@tauri-apps/plugin-dialog';
 * import { appDir } from '@tauri-apps/api/path';
 * // Open a selection dialog for directories
 * const selected = await open({
 *   directory: true,
 *   multiple: true,
 *   defaultPath: await appDir(),
 * });
 * if (Array.isArray(selected)) {
 *   // user selected multiple directories
 * } else if (selected === null) {
 *   // user cancelled the selection
 * } else {
 *   // user selected a single directory
 * }
 * ```
 *
 * @returns A promise resolving to the selected path(s)
 *
 * @since 2.0.0
 */
async function open(options = {}) {
    if (typeof options === 'object') {
        Object.freeze(options);
    }
    return await invoke('plugin:dialog|open', { options });
}

const VERSION = "20260228-1";
let searchResults = [];
let selectedComponent = null;
let currentSource = "easyeda";

console.log("JLC2KiCad version:", VERSION);

document.addEventListener("DOMContentLoaded", async () => {
  try {
    initTabs();
    await init();
  } catch (error) {
    console.error("初始化失败", error);
  } finally {
    setupEventListeners();
  }
});

function initTabs() {
  const tabBtns = document.querySelectorAll(".tab-btn");
  tabBtns.forEach(btn => {
    btn.addEventListener("click", () => {
      const tab = btn.dataset.tab;
      
      tabBtns.forEach(b => b.classList.remove("active"));
      btn.classList.add("active");
      
      document.querySelectorAll(".tab-panel").forEach(p => p.classList.remove("active"));
      document.getElementById(`panel-${tab}`).classList.add("active");
      
      currentSource = tab;
      hideResults();
    });
  });
}

async function init() {
  try {
    const defaultDir = await invoke("get_default_output_dir");
    document.getElementById("outputDir").value = defaultDir;
  } catch (e) {
    console.log("Using default output directory");
  }

  await loadNetworkSettings();
}

function setupEventListeners() {
  document.getElementById("easyedaInput").addEventListener("keypress", (e) => {
    if (e.key === "Enter") searchEasyEDA();
  });
  
  document.getElementById("lcscInput").addEventListener("keypress", (e) => {
    if (e.key === "Enter") searchLCSC();
  });
}

function showStatus(msg) {
  const statusDiv = document.getElementById("status");
  const log = document.getElementById("progressLog");
  const p = document.createElement("p");
  p.textContent = msg;
  log.appendChild(p);
  statusDiv.classList.remove("hidden");
}

function hideStatus() {
  document.getElementById("status").classList.add("hidden");
}

function showMessage(msg, isError = false) {
  const div = document.getElementById("resultMessage");
  div.textContent = msg;
  div.className = `result-message ${isError ? "error" : "success"}`;
  div.classList.remove("hidden");
}

function hideMessage() {
  document.getElementById("resultMessage").classList.add("hidden");
}

function hideResults() {
  document.getElementById("resultsSection").classList.add("hidden");
  document.getElementById("selectedPart").textContent = "-";
  hideMessage();
  selectedComponent = null;
}

function showResults(items) {
  const list = document.getElementById("resultsList");
  list.innerHTML = "";
  
  if (items.length === 0) {
    list.innerHTML = "<p style='color: var(--text-secondary)'>未找到相关零件</p>";
  } else {
    items.forEach((item, index) => {
      const div = document.createElement("div");
      div.className = "result-item";
      
      let thumbHtml = '';
      if (item.image_url) {
        thumbHtml = `<img class="part-img" src="${item.image_url}" alt="" onerror="this.style.display='none';this.nextSibling.style.display='inline'"/><img class="part-icon" src="assets/icons/default.png" style="display:none"/>`;
      } else {
        const pkg = (item.package || '').toUpperCase();
        let iconName = 'default.png';
        
        if (pkg.includes('BGA')) {
          iconName = 'bga.png';
        } else if (pkg.includes('QFN') || pkg.includes('DFN') || pkg.includes('TSSOP')) {
          iconName = 'qfn.png';
        } else if (pkg.includes('SOP') || pkg.includes('SOIC') || pkg.includes('SOT')) {
          iconName = 'sop.png';
        } else if (pkg.includes('DIP') || pkg.includes('双列直插')) {
          iconName = 'dip.png';
        } else if (pkg.includes('CAP-SMD') || pkg.includes('贴片电容')) {
          iconName = 'capacitor-smd.png';
        } else if (pkg.includes('CAP-TH') || pkg.includes('电解电容')) {
          iconName = 'capacitor-th.png';
        } else if (pkg.includes('RES-SMD') || pkg.includes('贴片电阻')) {
          iconName = 'resistor-smd.png';
        } else if (pkg.includes('RES-TH') || pkg.includes('直插电阻')) {
          iconName = 'resistor-th.png';
        } else if (pkg.includes('RES') || pkg.includes('电阻')) {
          iconName = 'resistor-smd.png';
        } else if (pkg.includes('LED')) {
          iconName = 'led.png';
        } else if (pkg.includes('CONN') || pkg.includes('连接器')) {
          iconName = 'connector.png';
        } else if (pkg.includes('DIODE') || pkg.includes('二极管')) {
          iconName = 'diode.png';
        } else if (pkg.includes('TRANS') || pkg.includes('三极管') || pkg.includes('TO-252') || pkg.includes('TO-263')) {
          iconName = 'transistor.png';
        } else if (pkg.includes('MOS')) {
          iconName = 'mos.png';
        } else if (pkg.includes('IND') || pkg.includes('电感')) {
          iconName = 'inductor.png';
        } else if (pkg.includes('CRYSTAL') || pkg.includes('晶振')) {
          iconName = 'crystal.png';
        }
        
        thumbHtml = `<img class="part-icon" src="assets/icons/${iconName}" alt=""/>`;
      }
      
      div.innerHTML = `
        ${thumbHtml}
        <div class="part-info">
          <span class="part-id">${item.id}</span>
          <span class="part-name">${item.name}</span>
        </div>
        <span class="part-desc">${item.description || ''}</span>
      `;
      div.onclick = () => selectComponent(index);
      list.appendChild(div);
    });
  }
       
  document.getElementById("resultsSection").classList.remove("hidden");
}

function selectComponent(index) {
  document.querySelectorAll(".result-item").forEach((item, i) => {
    item.classList.toggle("selected", i === index);
  });

  selectedComponent = searchResults[index];
  
  document.getElementById("selectedPart").textContent = `${selectedComponent.id} - ${selectedComponent.name}`;
}

async function searchEasyEDA() {
  const input = document.getElementById("easyedaInput").value.trim();
  if (!input) {
    showMessage("请输入零件编号", true);
    return;
  }

  hideResults();
  showStatus("正在搜索...");

  try {
    const results = await invoke("search_easyeda_cmd", { query: input });
    searchResults = results;
    hideStatus();
    showResults(results);
  } catch (error) {
    hideStatus();
    showMessage(`搜索失败: ${error}`, true);
  }
}

async function searchLCSC() {
  const input = document.getElementById("lcscInput").value.trim();
  if (!input) {
    showMessage("请输入LCSC编号", true);
    return;
  }

  hideResults();
  showStatus("正在搜索立创商城...");

  try {
    const results = await invoke("search_lcsc", { query: input });
    searchResults = results;
    hideStatus();
    showResults(results);
  } catch (error) {
    hideStatus();
    showMessage(`搜索失败: ${error}`, true);
  }
}

async function selectFolder() {
  try {
    const selected = await open({
      directory: false,
      multiple: false,
      title: "选择元器件文件"
    });
    
    if (selected) {
      document.getElementById("localPath").value = selected;
      
      showStatus("正在加载本地数据...");
      const results = await invoke("load_local_folder", { path: selected });
      searchResults = results;
      hideStatus();
      showResults(results);
    }
  } catch (error) {
    hideStatus();
    showMessage(`选择文件夹失败: ${error}`, true);
  }
}

async function exportOne(type) {
  if (!selectedComponent) {
    showMessage("请先选择一个零件", true);
    return;
  }

  const outputDir = document.getElementById("outputDir").value.trim();
  const symbolLib = document.getElementById("symbolLib").value.trim();
  const footprintLib = document.getElementById("footprintLib").value.trim();

  hideMessage();
  showStatus("正在导出...");

  try {
    let result;
    
    if (type === 'symbol') {
      // 导出器件库
      if (currentSource === "local") {
        result = await invoke("convert_local", {
          options: {
            path: document.getElementById("localPath").value,
            output_dir: outputDir,
            footprint_lib: footprintLib,
            symbol_lib: symbolLib,
            symbol_path: "symbol",
            model_dir: "packages3d",
            models: [],
            create_footprint: false,
            create_symbol: true,
          },
        });
      } else {
        result = await invoke("create_component_cmd", {
          options: {
            component_id: selectedComponent.id,
            output_dir: outputDir,
            footprint_lib: footprintLib,
            symbol_lib: symbolLib,
            symbol_path: "symbol",
            model_dir: "packages3d",
            models: [],
            create_footprint: false,
            create_symbol: true,
          },
        });
      }
    } else if (type === 'footprint') {
      // 导出封装库
      if (currentSource === "local") {
        result = await invoke("convert_local", {
          options: {
            path: document.getElementById("localPath").value,
            output_dir: outputDir,
            footprint_lib: footprintLib,
            symbol_lib: symbolLib,
            symbol_path: "symbol",
            model_dir: "packages3d",
            models: [],
            create_footprint: true,
            create_symbol: false,
          },
        });
      } else {
        result = await invoke("create_component_cmd", {
          options: {
            component_id: selectedComponent.id,
            output_dir: outputDir,
            footprint_lib: footprintLib,
            symbol_lib: symbolLib,
            symbol_path: "symbol",
            model_dir: "packages3d",
            models: [],
            create_footprint: true,
            create_symbol: false,
          },
        });
      }
    } else if (type === '3d') {
      // 导出3D模型
      if (currentSource === "local") {
        result = await invoke("convert_local", {
          options: {
            path: document.getElementById("localPath").value,
            output_dir: outputDir,
            footprint_lib: footprintLib,
            symbol_lib: symbolLib,
            symbol_path: "symbol",
            model_dir: "packages3d",
            models: ["STEP"],
            create_footprint: false,
            create_symbol: false,
          },
        });
      } else {
        result = await invoke("create_component_cmd", {
          options: {
            component_id: selectedComponent.id,
            output_dir: outputDir,
            footprint_lib: footprintLib,
            symbol_lib: symbolLib,
            symbol_path: "symbol",
            model_dir: "packages3d",
            models: ["STEP"],
            create_footprint: false,
            create_symbol: false,
          },
        });
      }
    }

    hideStatus();

    if (result.success) {
      showMessage(`✅ ${result.message}`);
    } else {
      showMessage(`❌ ${result.error || "导出失败"}`, true);
    }
  } catch (error) {
    hideStatus();
    showMessage(`❌ 导出失败: ${error}`, true);
  }
}

listen("progress", (event) => {
  const log = document.getElementById("progressLog");
  const p = document.createElement("p");
  p.textContent = event.payload;
  log.appendChild(p);
}).catch((error) => {
  console.warn("progress event listen disabled:", error);
});

async function loadNetworkSettings() {
  try {
    const settings = await invoke("get_network_settings_cmd");
    document.getElementById("easyedaUseProxy").checked = !!settings.easyeda_use_proxy;
    document.getElementById("lcscUseProxy").checked = !!settings.lcsc_use_proxy;
    document.getElementById("proxyAddress").value = settings.proxy_address || "";
  } catch (error) {
    console.error("加载网络设置失败", error);
  }
}

function toggleNetworkSettings() {
  const panel = document.getElementById("networkSettingsPanel");
  if (!panel) {
    showMessage("网络设置面板未找到", true);
    return;
  }
  panel.classList.toggle("hidden");
}

async function saveNetworkSettings() {
  const settings = {
    easyeda_use_proxy: document.getElementById("easyedaUseProxy").checked,
    lcsc_use_proxy: document.getElementById("lcscUseProxy").checked,
    proxy_address: document.getElementById("proxyAddress").value.trim(),
  };

  try {
    const result = await invoke("set_network_settings_cmd", { settings });
    if (result.success) {
      showMessage("网络设置已保存");
      toggleNetworkSettings();
    } else {
      showMessage(`保存失败: ${result.error || "未知错误"}`, true);
    }
  } catch (error) {
    showMessage(`保存失败: ${error}`, true);
  }
}

// Expose handlers for inline onclick bindings in index.html.
window.searchEasyEDA = searchEasyEDA;
window.searchLCSC = searchLCSC;
window.selectFolder = selectFolder;
window.exportOne = exportOne;
window.toggleNetworkSettings = toggleNetworkSettings;
window.saveNetworkSettings = saveNetworkSettings;
