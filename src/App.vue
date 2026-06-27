<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { LeftOutlined, RightOutlined } from "@ant-design/icons-vue";
import { useDhcpServerStore } from "./stores/dhcpServerStore";

/**
 * 应用主壳组件。
 *
 * 这一层只负责三件事：
 * 1. 展示顶部菜单栏；
 * 2. 维护一个轻量级的页面访问历史，支持前进 / 后退；
 * 3. 渲染当前路由页面。
 *
 * 业务页面本身放到各自的路由组件中处理，避免这里承担过多业务逻辑。
 */
const router = useRouter();
const route = useRoute();
const { dhcpServerRunning } = useDhcpServerStore();

/**
 * 当前选中的菜单 key。
 *
 * 我们优先使用路由 name，这样即使未来页面路径调整，
 * 只要路由 name 不变，菜单高亮逻辑也不需要跟着改。
 */
const currentMenu = computed(() => String(route.name ?? ""));

/**
 * 轻量历史栈。
 *
 * 这里不是替代浏览器历史，而是为了配合顶部自定义前进 / 后退按钮，
 * 让应用内部跳转体验更像桌面工具。
 */
const historyStack = ref<string[]>([]);
const historyIndex = ref(-1);
const canGoBack = ref(false);
const canGoForward = ref(false);

/**
 * 根据当前索引刷新前进 / 后退按钮状态。
 */
function syncHistoryAbility() {
  canGoBack.value = historyIndex.value > 0;
  canGoForward.value = historyIndex.value >= 0 && historyIndex.value < historyStack.value.length - 1;
}

/**
 * 监听路由变化，把外部触发的跳转也同步到自定义历史栈中。
 *
 * 例如：
 * - 用户点击浏览器后退；
 * - 代码里主动调用 router.push；
 * - 应用首次进入默认页面。
 */
watch(
  () => route.name,
  (routeName) => {
    const key = String(routeName ?? "");
    if (!key) {
      return;
    }

    /**
     * 如果当前历史指针已经在该页面，就不重复记录。
     */
    if (historyStack.value[historyIndex.value] === key) {
      syncHistoryAbility();
      return;
    }

    /**
     * 如果新页面恰好是历史栈里已有的相邻项，
     * 说明这次更像是前进 / 后退行为，直接移动指针即可。
     */
    const previousKey = historyStack.value[historyIndex.value - 1];
    const nextKey = historyStack.value[historyIndex.value + 1];

    if (key === previousKey) {
      historyIndex.value -= 1;
      syncHistoryAbility();
      return;
    }

    if (key === nextKey) {
      historyIndex.value += 1;
      syncHistoryAbility();
      return;
    }

    /**
     * 其余情况按“新导航”处理：
     * - 如果之前后退过，需要先截断后面的分支历史；
     * - 再把当前页面压入历史栈尾部。
     */
    if (historyIndex.value < historyStack.value.length - 1) {
      historyStack.value = historyStack.value.slice(0, historyIndex.value + 1);
    }

    historyStack.value.push(key);
    historyIndex.value = historyStack.value.length - 1;
    syncHistoryAbility();
  },
  { immediate: true },
);

/**
 * 菜单点击处理。
 *
 * 这里直接按路由 name 导航，页面之间的具体参数暂不涉及。
 */
function handleMenuClick(menuItem: { key: string }) {
  router.push({ name: menuItem.key });
}

/**
 * 返回历史中的上一页。
 */
function goBack() {
  if (!canGoBack.value) {
    return;
  }

  historyIndex.value -= 1;
  syncHistoryAbility();
  router.push({ name: historyStack.value[historyIndex.value] });
}

/**
 * 前进到历史中的下一页。
 */
function goForward() {
  if (!canGoForward.value) {
    return;
  }

  historyIndex.value += 1;
  syncHistoryAbility();
  router.push({ name: historyStack.value[historyIndex.value] });
}
</script>

<template>
  <div class="app-layout">
    <header class="app-header">
      <h1>Camera Tools</h1>
    </header>

    <div class="app-menu">
      <div class="menu-inner">
        <a-menu
          mode="horizontal"
          :selectedKeys="[currentMenu]"
          :overflowedIndicator="null"
          triggerSubMenuAction="click"
          @click="handleMenuClick"
        >
          <!-- "文件"顶级入口 -->
          <a-menu-item key="file">
            文件
          </a-menu-item>

          <a-sub-menu key="pages" title="页面">
            <a-menu-item key="someip-player">
              SOME/IP Player
            </a-menu-item>
          </a-sub-menu>

          <a-sub-menu key="tools" title="工具">
            <a-menu-item key="dhcp-server">
              <span
                class="status-dot"
                :class="dhcpServerRunning ? 'dot-green' : 'dot-red'"
              ></span>
              DHCP Server
            </a-menu-item>
          </a-sub-menu>

          <!-- SSH 菜单，放在"工具"和"帮助"之间 -->
          <a-menu-item key="ssh">
            SSH
          </a-menu-item>

          <a-menu-item key="help">
            帮助
          </a-menu-item>
        </a-menu>

        <div class="right-actions">
          <div class="nav-buttons">
            <a-button
              class="nav-btn"
              :disabled="!canGoBack"
              @click="goBack"
            >
              <LeftOutlined />
            </a-button>

            <a-button
              class="nav-btn"
              :disabled="!canGoForward"
              @click="goForward"
            >
              <RightOutlined />
            </a-button>
          </div>

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
      <router-view />
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.app-header {
  display: flex;
  align-items: center;
  height: 48px;
  padding: 0 24px;
  background-color: #001529;
  color: #fff;
}

.app-header h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.app-menu {
  border-bottom: 1px solid #f0f0f0;
}

.menu-inner {
  display: flex;
  align-items: center;
}

.menu-inner .ant-menu {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.right-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-right: 16px;
}

.nav-buttons {
  display: flex;
  gap: 4px;
}

.nav-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 40px;
  height: 32px;
  padding: 0 10px;
  border: none;
  border-radius: 4px;
  background: #333;
  color: #fff;
  cursor: pointer;
  transition: all 0.2s;
}

.nav-btn:hover:not(:disabled) {
  background: #555;
  color: #fff;
}

.nav-btn:disabled {
  background: #e0e0e0;
  color: #bbb;
  cursor: not-allowed;
}

.nav-btn .anticon {
  font-size: 14px;
}

.status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  margin-right: 6px;
  border-radius: 50%;
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

.theme-selector {
  min-width: 120px;
}

.app-content {
  flex: 1;
  padding: 24px;
}
</style>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}
</style>