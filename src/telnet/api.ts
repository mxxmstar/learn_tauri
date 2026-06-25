/**
 * Telnet 模块前端 API 封装
 *
 * 提供与后端 Tauri 命令交互的函数
 */

import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import type {
  TelnetConfig,
  LoginResult,
  CommandResult,
  FileDownloadResult,
  DownloadProgress,
  ConnectionStatus,
  TelnetCmdResult,
} from './types';

/**
 * 连接设备
 *
 * @param config - Telnet 配置
 * @returns 操作结果
 *
 * @example
 * ```typescript
 * const result = await connect({
 *   addr: '192.168.1.1:23',
 *   connectTimeoutMs: 10000,
 *   loginTimeoutMs: 15000,
 *   commandTimeoutMs: 30000,
 * });
 * ```
 */
export async function connect(
  config: TelnetConfig
): Promise<TelnetCmdResult<void>> {
  return invoke('telnet_connect', { config });
}

/**
 * 登录设备
 *
 * @param username - 用户名
 * @param password - 密码
 * @returns 登录结果
 *
 * @example
 * ```typescript
 * const result = await login('root', 'password');
 * if (result.success) {
 *   console.log('登录成功');
 * }
 * ```
 */
export async function login(
  username: string,
  password: string
): Promise<TelnetCmdResult<LoginResult>> {
  return invoke('telnet_login', { username, password });
}

/**
 * 执行命令
 *
 * @param command - 要执行的命令
 * @returns 命令执行结果
 *
 * @example
 * ```typescript
 * const result = await sendCommand('ls -la');
 * if (result.success) {
 *   console.log(result.data?.output);
 * }
 * ```
 */
export async function sendCommand(
  command: string
): Promise<TelnetCmdResult<CommandResult>> {
  return invoke('telnet_send_command', { command });
}

/**
 * 下载文件
 *
 * @param remotePath - 远程文件路径
 * @param localPath - 本地保存路径
 * @param onProgress - 进度回调函数（可选）
 * @returns 下载结果
 *
 * @example
 * ```typescript
 * const result = await downloadFile(
 *   '/etc/config',
 *   'C:\\Users\\Downloads\\config.txt',
 *   (progress) => {
 *     console.log(`下载进度: ${progress.progress * 100}%`);
 *   }
 * );
 * ```
 */
export async function downloadFile(
  remotePath: string,
  localPath: string,
  onProgress?: (progress: DownloadProgress) => void
): Promise<TelnetCmdResult<FileDownloadResult>> {
  // 如果提供了进度回调，监听进度事件
  let unlisten: (() => void) | undefined;
  
  if (onProgress) {
    listen<DownloadProgress>('telnet-download-progress', (event) => {
      onProgress(event.payload);
    }).then((unlistenFn) => {
      unlisten = unlistenFn;
    });
  }

  try {
    const result = await invoke<TelnetCmdResult<FileDownloadResult>>(
      'telnet_download_file',
      { remotePath, localPath }
    );
    return result;
  } finally {
    // 清理事件监听器
    if (unlisten) {
      unlisten();
    }
  }
}

/**
 * 断开连接
 *
 * @returns 操作结果
 *
 * @example
 * ```typescript
 * const result = await disconnect();
 * ```
 */
export async function disconnect(): Promise<TelnetCmdResult<void>> {
  return invoke('telnet_disconnect');
}

/**
 * 获取连接状态
 *
 * @returns 当前连接状态
 *
 * @example
 * ```typescript
 * const status = await getStatus();
 * console.log(status.data); // 'Disconnected' | 'Connecting' | ...
 * ```
 */
export async function getStatus(): Promise<TelnetCmdResult<ConnectionStatus>> {
  return invoke('telnet_get_status');
}

/**
 * 默认配置
 */
export const DEFAULT_CONFIG: TelnetConfig = {
  addr: '192.168.1.1:23',
  connectTimeoutMs: 10000,
  loginTimeoutMs: 15000,
  commandTimeoutMs: 30000,
  loginPrompt: 'login:',
  passwordPrompt: 'Password:',
  prompts: ['# ', '$ ', '> '],
  bufferSize: 8192,
};
