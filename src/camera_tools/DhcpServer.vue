<script setup lang="ts">
import { useDhcpServerStore } from "../stores/dhcpServerStore";

const { dhcpServerRunning, toggleDhcpServer } = useDhcpServerStore();
</script>

<template>
  <div class="dhcp-server">
    <!-- 页面标题 -->
    <h2>DHCP Server</h2>

    <div class="page-body">
      <!-- ====== 左侧：服务控制区 ====== -->
      <div class="left-panel">
        <!-- 服务状态卡片 -->
        <div class="status-card">
          <div class="status-info">
            <!-- 状态指示灯：与服务子菜单联动 -->
            <span
              class="status-dot-lg"
              :class="dhcpServerRunning ? 'dot-green' : 'dot-red'"
            ></span>
            <span>
              DHCP 服务器当前状态：
              <strong :class="dhcpServerRunning ? 'text-green' : 'text-red'">
                {{ dhcpServerRunning ? "运行中" : "已停止" }}
              </strong>
            </span>
          </div>

          <!-- 开关控制 -->
          <a-switch
            :checked="dhcpServerRunning"
            :checked-children="'运行'"
            :un-checked-children="'停止'"
            @change="toggleDhcpServer"
          />
        </div>

        <p class="desc">切换上方开关，左侧菜单栏 "DHCP Server" 前的指示灯会同步变化。</p>
      </div>

      <!-- ====== 右侧：设备交互日志窗口 ====== -->
      <div class="right-panel">
        <div class="interaction-panel">
          <div class="panel-header">
            <span class="panel-title">设备交互</span>
          </div>
          <div class="panel-body">
            <p class="placeholder-text">暂无交互记录</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dhcp-server {
  padding: 16px;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.dhcp-server h2 {
  margin-bottom: 16px;
  font-size: 20px;
  font-weight: 600;
  flex-shrink: 0;
}

/* ====== 左右分栏布局 ====== */
.page-body {
  display: flex;
  gap: 16px;
  flex: 1;
  min-height: 0;
}

/* 左侧：服务控制区 */
.left-panel {
  flex: 1;
  min-width: 0;
}

/* 右侧：设备交互窗口 */
.right-panel {
  width: 360px;
  flex-shrink: 0;
}

/* ====== 设备交互面板 ====== */
.interaction-panel {
  height: 100%;
  border: 1px solid #f0f0f0;
  border-radius: 8px;
  background: #fff;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.interaction-panel .panel-header {
  padding: 10px 16px;
  border-bottom: 1px solid #f0f0f0;
  background: #fafafa;
}

.interaction-panel .panel-title {
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.interaction-panel .panel-body {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
}

.placeholder-text {
  color: #bbb;
  font-size: 13px;
  text-align: center;
  margin-top: 60px;
}

/* ====== 状态卡片 ====== */
.status-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border: 1px solid #f0f0f0;
  border-radius: 8px;
  background: #fafafa;
  margin-bottom: 12px;
}

/* 状态信息区 */
.status-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}

/* 大号状态指示灯 */
.status-dot-lg {
  display: inline-block;
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.status-dot-lg.dot-green {
  background-color: #52c41a;
  box-shadow: 0 0 6px rgba(82, 196, 26, 0.6);
}

.status-dot-lg.dot-red {
  background-color: #ff4d4f;
  box-shadow: 0 0 6px rgba(255, 77, 79, 0.6);
}

.text-green {
  color: #52c41a;
}

.text-red {
  color: #ff4d4f;
}

.desc {
  color: #888;
  font-size: 13px;
}
</style>