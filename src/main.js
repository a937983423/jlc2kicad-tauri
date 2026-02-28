import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

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
    const selected = await openDialog({
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
