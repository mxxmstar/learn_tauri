<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useRouter, useRoute } from "vue-router";
import { LeftOutlined, RightOutlined } from "@ant-design/icons-vue";
import { useDhcpServerStore } from "./stores/dhcpServerStore";

const router = useRouter();
const route = useRoute();
const { dhcpServerRunning } = useDhcpServerStore();

/**
 * 当前选中的菜单项 key（从路由路径中提取首段）
 * 例如 /dhcp-server → dhcp-server
 */
const currentMenu = computed(() => route.path.slice(1));

// 历史记录栈，用于自定义前进/后退导航
const history = ref<string[]>([]);
const historyIndex = ref(-1);
const canGoBack = ref(false);
const canGoForward = ref(false);

/**
 * 监听路由变化，当使用浏览器/按钮前进/后退时，同步更新历史栈状态
 */
watch(
  () => route.path,
  (path) => {
    const key = path.slice(1);
    // 只有当路由变化不是由 handleMenuClick 触发时才同步（避免重复记录）
    if (history.value[historyIndex.value] !== key) {
      // 如果是通过 goBack/goForward 触发的，历史栈中应该已有该 key
      const idx = history.value.indexOf(key);
      if (idx !== -1) {
        historyIndex.value = idx;
      } else {
        // 未知导航（如直接输入 URL），重置历史栈
        history.value = [key];
        historyIndex.value = 0;
      }
    }
    canGoBack.value = historyIndex.value > 0;
    canGoForward.value = historyIndex.value < history.value.length - 1;
  },
);

/**
 * 点击菜单项时触发：记录历史 + 路由跳转
 * @param menuItem - 点击的菜单项对象，包含 key 属性
 */
const handleMenuClick = (menuItem: { key: string }) => {
  const key = menuItem.key;

  // 如果当前不在历史栈末尾（即已经后退过），则丢弃当前位置之后的记录
  if (historyIndex.value < history.value.length - 1) {
    history.value = history.value.slice(0, historyIndex.value + 1);
  }
  // 将新菜单项追加到历史栈，并更新索引
  history.value.push(key);
  historyIndex.value = history.value.length - 1;
  canGoBack.value = historyIndex.value > 0;
  canGoForward.value = false;

  // 通过 Vue Router 跳转
  router.push({ name: key });
};

/** 后退：回到历史记录中的上一个页面 */
const goBack = () => {
  if (historyIndex.value > 0) {
    historyIndex.value--;
    const key = history.value[historyIndex.value];
    canGoBack.value = historyIndex.value > 0;
    canGoForward.value = true;
    router.push({ name: key });
  }
};

/** 前进：回到历史记录中的下一个页面 */
const goForward = () => {
  if (historyIndex.value < history.value.length - 1) {
    historyIndex.value++;
    const key = history.value[historyIndex.value];
    canGoForward.value = historyIndex.value < history.value.length - 1;
    canGoBack.value = true;
    router.push({ name: key });
  }
};
</script>

<template>
  <div class="app-layout">
    <header class="app-header">
      <!-- 应用标题 -->
      <h1>Camera Tools</h1>
    </header>

    <div class="app-menu">
      <div class="menu-inner">
        <!-- 左侧导航菜单 -->
        <a-menu
          mode="horizontal"
          :selectedKeys="[currentMenu]"
          @click="handleMenuClick"
          :overflowedIndicator="null"
          triggerSubMenuAction="click"
        >
          <a-menu-item key="file">
            文件
          </a-menu-item>
          <!-- "页面"作为子菜单，点击弹出下拉选项 -->
          <a-sub-menu key="pages" title="页面">
            <a-menu-item key="someip-player">
              SOME/IP Player
            </a-menu-item>
          </a-sub-menu>
          <!-- "工具"作为子菜单，点击弹出下拉选项 -->
          <a-sub-menu key="tools" title="工具">
            <a-menu-item key="dhcp-server">
              <!-- 运行状态指示灯：绿色=运行中，红色=已停止 -->
              <span
                class="status-dot"
                :class="dhcpServerRunning ? 'dot-green' : 'dot-red'"
              ></span>
              DHCP Server
            </a-menu-item>
          </a-sub-menu>
          <a-menu-item key="help">
            帮助
          </a-menu-item>
        </a-menu>

        <!-- 右侧操作区：导航按钮 + 主题选择 -->
        <div class="right-actions">
          <div class="nav-buttons">
            <!-- 后退按钮 -->
            <a-button
              class="nav-btn"
              :disabled="!canGoBack"
              @click="goBack"
            >
              <LeftOutlined />
            </a-button>
            <!-- 前进按钮 -->
            <a-button
              class="nav-btn"
              :disabled="!canGoForward"
              @click="goForward"
            >
              <RightOutlined />
            </a-button>
          </div>
          <!-- 主题选择下拉框 -->
          <a-select
            class="theme-selector"
            :value="'default'"
            :style="{ width: '120px' }"
          >
            <a-select-option value="default">
              默认主题
            </a-select-option>
          </a-select>
        </div>
      </div>
    </div>

    <main class="app-content">
      <!-- Vue Router 页面渲染出口 -->
      <router-view />
    </main>
  </div>
</template>

<style scoped>
/* ========== 整体布局 ========== */
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* ========== 标题栏 ========== */
.app-header {
  background-color: #001529;
  color: #fff;
  padding: 0 24px;
  display: flex;
  align-items: center;
  height: 48px;
}

.app-header h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

/* ========== 菜单栏 ========== */
.app-menu {
  border-bottom: 1px solid #f0f0f0;
}

/* 菜单栏内部容器：左侧菜单 + 右侧操作区 */
.menu-inner {
  display: flex;
  align-items: center;
}

/* 左侧菜单占满剩余空间，防止 ant-design 自动溢出折叠为 "..." */
.menu-inner .ant-menu {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

/* 右侧操作区：导航按钮 + 主题选择器 */
.right-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-right: 16px;
}

/* ========== 导航按钮 ========== */
.nav-buttons {
  display: flex;
  gap: 4px;
}

.nav-btn {
  /* 长方形按钮：固定高度，宽度由 padding 撑开 */
  height: 32px;
  min-width: 40px;
  padding: 0 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 4px;
  background: #333;
  color: #fff;
  cursor: pointer;
  transition: all 0.2s;
}

/* 悬停时：背景变浅 */
.nav-btn:hover:not(:disabled) {
  background: #555;
  color: #fff;
}

/* 禁用时：灰色背景 + 浅灰图标 */
.nav-btn:disabled {
  background: #e0e0e0;
  color: #bbb;
  cursor: not-allowed;
}

/* 图标尺寸 */
.nav-btn .anticon {
  font-size: 14px;
}

/* ========== 状态指示灯 ========== */
.status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 6px;
  vertical-align: middle;
}

.dot-green {
  background-color: #52c41a;
  box-shadow: 0 0 4px rgba(82, 196, 26, 0.6);
}

.dot-red {
  background-color: #ff4d4f;
  box-shadow: 0 0 4px rgba(255, 77, 79, 0.6);
}

/* ========== 主题选择器 ========== */
.theme-selector {
  min-width: 120px;
}

/* ========== 内容区域 ========== */
.app-content {
  flex: 1;
  padding: 24px;
}
</style>
<style>
/* ========== 全局重置样式 ========== */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}
</style>