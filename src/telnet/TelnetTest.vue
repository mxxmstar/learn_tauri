<template>
  <div class="telnet-test">
    <t-card title="Telnet 连接测试" bordered>
      <!-- 连接配置 -->
      <t-form :data="config" layout="inline" style="margin-bottom: 16px">
        <t-form-item label="地址">
          <t-input v-model="config.addr" placeholder="192.168.1.1:23" />
        </t-form-item>
        <t-form-item label="用户名">
          <t-input v-model="username" placeholder="root" />
        </t-form-item>
        <t-form-item label="密码">
          <t-input v-model="password" type="password" placeholder="密码" />
        </t-form-item>
        <t-form-item>
          <t-space>
            <t-button
              theme="primary"
              :loading="connecting"
              @click="handleConnect"
            >
              连接
            </t-button>
            <t-button
              theme="warning"
              :loading="logging"
              @click="handleLogin"
            >
              登录
            </t-button>
            <t-button
              theme="danger"
              :loading="disconnecting"
              @click="handleDisconnect"
            >
              断开
            </t-button>
          </t-space>
        </t-form-item>
      </t-form>

      <!-- 连接状态 -->
      <t-alert
        :theme="statusTheme"
        style="margin-bottom: 16px"
      >
        连接状态: {{ statusText }}
      </t-alert>

      <!-- 命令输入 -->
      <t-form layout="inline" style="margin-bottom: 16px">
        <t-form-item label="命令">
          <t-input
            v-model="command"
            placeholder="输入命令 (如: ls -la)"
            style="width: 400px"
            @keyup.enter="handleSendCommand"
          />
        </t-form-item>
        <t-form-item>
          <t-button
            theme="primary"
            :loading="commanding"
            @click="handleSendCommand"
          >
            发送
          </t-button>
        </t-form-item>
      </t-form>

      <!-- 命令输出 -->
      <t-card title="命令输出" bordered style="margin-bottom: 16px">
        <t-textarea
          :value="output"
          readonly
          :autosize="{ minRows: 10, maxRows: 20 }"
        />
      </t-card>

      <!-- NFS 挂载 -->
      <t-divider />
      <t-form layout="inline">
        <t-form-item label="VM IP">
          <t-input v-model="vmIp" placeholder="192.168.66.11" style="width: 140px" />
        </t-form-item>
        <t-form-item label="挂载路径">
          <t-input v-model="mountPath" placeholder="/mnt/nfs" style="width: 140px" />
        </t-form-item>
        <t-form-item>
          <t-button theme="primary" :loading="mounting" @click="handleMount">
            挂载
          </t-button>
        </t-form-item>
      </t-form>

      <!-- 文件下载 -->
      <t-form layout="inline">
        <t-form-item label="远程路径">
          <t-input
            v-model="remotePath"
            placeholder="/etc/config"
            style="width: 200px"
          />
        </t-form-item>
        <t-form-item label="本地路径">
          <t-input
            v-model="localPath"
            placeholder="C:\Downloads\config.txt"
            style="width: 250px"
          />
        </t-form-item>
        <t-form-item>
          <t-button
            theme="primary"
            :loading="downloading"
            @click="handleDownload"
          >
            下载
          </t-button>
        </t-form-item>
      </t-form>
    </t-card>
  </div>
</template>

<script setup lang="ts">
// @ts-nocheck
import { ref, computed, onMounted } from 'vue';
import { MessagePlugin } from 'tdesign-vue-next';
import {
  connect,
  login,
  sendCommand,
  downloadFile,
  disconnect,
  getStatus,
  mountVm,
  DEFAULT_CONFIG,
  type TelnetConfig,
  type ConnectionStatus,
} from './api';

// 连接配置
const config = ref<TelnetConfig>({
  ...DEFAULT_CONFIG,
  addr: '192.168.66.218:23',
});

// 登录信息
const username = ref('root');
const password = ref('better@ADA32');

// 命令相关
const command = ref('');
const output = ref('');
const connecting = ref(false);
const logging = ref(false);
const disconnecting = ref(false);
const commanding = ref(false);
const downloading = ref(false);
const currentStatus = ref<ConnectionStatus>('Disconnected');

// NFS 挂载
const vmIp = ref('192.168.66.11');
const mountPath = ref('/mnt/nfs');
const mounting = ref(false);

// 文件下载
const remotePath = ref('/etc/hostname');
const localPath = ref('C:\\Downloads\\hostname.txt');

// 状态显示
const statusTheme = computed(() => {
  switch (currentStatus.value) {
    case 'Connected':
      return 'success';
    case 'Connecting':
    case 'LoginInProgress':
      return 'warning';
    case 'LoginFailed':
    case 'LoginTimeout':
      return 'error';
    default:
      return 'info';
  }
});

const statusText = computed(() => {
  switch (currentStatus.value) {
    case 'Disconnected':
      return '未连接';
    case 'Connecting':
      return '连接中...';
    case 'Connected':
      return '已连接';
    case 'LoginInProgress':
      return '登录中...';
    case 'LoginFailed':
      return '登录失败';
    case 'LoginTimeout':
      return '登录超时';
    default:
      return currentStatus.value;
  }
});

// 更新状态
async function updateStatus() {
  try {
    const result = await getStatus();
    if (result.success && result.data) {
      currentStatus.value = result.data;
    }
  } catch (e) {
    console.error('获取状态失败:', e);
  }
}

// 连接
async function handleConnect() {
  connecting.value = true;
  try {
    const result = await connect(config.value);
    if (result.success) {
      MessagePlugin.success('连接成功');
      await updateStatus();
    } else {
      MessagePlugin.error(`连接失败: ${result.error}`);
    }
  } catch (e: any) {
    MessagePlugin.error(`连接异常: ${e.message || e}`);
  } finally {
    connecting.value = false;
  }
}

// 登录
async function handleLogin() {
  if (!username.value || !password.value) {
    MessagePlugin.warning('请输入用户名和密码');
    return;
  }

  logging.value = true;
  try {
    const result = await login(username.value, password.value);
    if (result.success && result.data) {
      MessagePlugin.success(result.data.message || '登录成功');
      await updateStatus();
    } else {
      MessagePlugin.error(`登录失败: ${result.error}`);
    }
  } catch (e: any) {
    MessagePlugin.error(`登录异常: ${e.message || e}`);
  } finally {
    logging.value = false;
  }
}

// 发送命令
async function handleSendCommand() {
  if (!command.value) {
    MessagePlugin.warning('请输入命令');
    return;
  }

  commanding.value = true;
  try {
    const result = await sendCommand(command.value);
    if (result.success && result.data) {
      output.value += `\n$ ${command.value}\n${result.data.output}`;
    } else {
      MessagePlugin.error(`命令执行失败: ${result.error}`);
    }
  } catch (e: any) {
    MessagePlugin.error(`命令执行异常: ${e.message || e}`);
  } finally {
    commanding.value = false;
  }
}

// 下载文件
async function handleDownload() {
  if (!remotePath.value || !localPath.value) {
    MessagePlugin.warning('请输入文件路径');
    return;
  }

  downloading.value = true;
  try {
    const result = await downloadFile(remotePath.value, localPath.value);
    if (result.success && result.data) {
      MessagePlugin.success(result.data.message || '下载成功');
    } else {
      MessagePlugin.error(`下载失败: ${result.error}`);
    }
  } catch (e: any) {
    MessagePlugin.error(`下载异常: ${e.message || e}`);
  } finally {
    downloading.value = false;
  }
}

// 挂载 VM
async function handleMount() {
  if (!vmIp.value) {
    MessagePlugin.warning('请输入 VM IP');
    return;
  }

  mounting.value = true;
  try {
    const result = await mountVm(vmIp.value, '/nfs', mountPath.value);
    if (result.success && result.data?.success) {
      output.value += `\n$ mount ${vmIp.value}:/nfs -> ${mountPath.value}\n${result.data.output}`;
      MessagePlugin.success('挂载成功');
    } else {
      output.value += `\n$ mount ${vmIp.value}:/nfs -> ${mountPath.value} (失败)\n${result.data?.output || result.error}`;
      MessagePlugin.error(`挂载失败: ${result.data?.error || result.error}`);
    }
  } catch (e: any) {
    MessagePlugin.error(`挂载异常: ${e.message || e}`);
  } finally {
    mounting.value = false;
  }
}

// 断开连接
async function handleDisconnect() {
  disconnecting.value = true;
  try {
    const result = await disconnect();
    if (result.success) {
      MessagePlugin.success('已断开连接');
      await updateStatus();
    } else {
      MessagePlugin.error(`断开失败: ${result.error}`);
    }
  } catch (e: any) {
    MessagePlugin.error(`断开异常: ${e.message || e}`);
  } finally {
    disconnecting.value = false;
  }
}

// 初始化
onMounted(() => {
  updateStatus();
});
</script>

<style scoped>
.telnet-test {
  padding: 16px;
}

.telnet-test :deep(.t-textarea__inner) {
  font-family: 'Courier New', monospace;
  font-size: 13px;
}
</style>
