/**
 * Telnet 模块前端类型定义
 *
 * 这些类型与后端的 Rust 类型对应
 */

/**
 * 连接状态
 */
export enum ConnectionStatus {
  Disconnected = 'Disconnected',
  Connecting = 'Connecting',
  Connected = 'Connected',
  LoginInProgress = 'LoginInProgress',
  LoginFailed = 'LoginFailed',
  LoginTimeout = 'LoginTimeout',
}

/**
 * Telnet 配置
 */
export interface TelnetConfig {
  /** 目标地址 (IP:端口) */
  addr: string;
  /** 连接超时(毫秒) */
  connectTimeoutMs?: number;
  /** 登录超时(毫秒) */
  loginTimeoutMs?: number;
  /** 命令执行超时(毫秒) */
  commandTimeoutMs?: number;
  /** 登录提示符 */
  loginPrompt?: string;
  /** 密码提示符 */
  passwordPrompt?: string;
  /** 命令提示符列表 */
  prompts?: string[];
  /** 读取缓冲区大小 */
  bufferSize?: number;
}

/**
 * 登录结果
 */
export interface LoginResult {
  /** 是否成功 */
  success: boolean;
  /** 登录后的提示符 */
  prompt: string;
  /** 登录过程中的输出信息 */
  output: string;
}

/**
 * 命令执行结果
 */
export interface CommandResult {
  /** 命令退出状态 */
  exitStatus?: number;
  /** 命令输出 */
  output: string;
  /** 执行耗时(毫秒) */
  durationMs: number;
}

/**
 * 文件下载结果
 */
export interface FileDownloadResult {
  /** 是否成功 */
  success: boolean;
  /** 远程文件路径 */
  remotePath: string;
  /** 本地保存路径 */
  localPath: string;
  /** 文件大小(字节) */
  fileSize: number;
  /** 下载耗时(毫秒) */
  durationMs: number;
  /** 错误信息 */
  error?: string;
}

/**
 * 下载进度通知
 */
export interface DownloadProgress {
  /** 远程文件路径 */
  remotePath: string;
  /** 已下载字节数 */
  downloadedBytes: number;
  /** 文件总大小(字节) */
  totalBytes: number;
  /** 下载进度(0.0 - 1.0) */
  progress: number;
  /** 当前阶段 */
  stage: 'checking' | 'downloading' | 'saving' | 'completed' | 'error';
  /** 状态消息 */
  message: string;
}

/**
 * Tauri 命令返回结果
 */
export interface TelnetCmdResult<T> {
  /** 是否成功 */
  success: boolean;
  /** 返回数据 */
  data?: T;
  /** 错误信息 */
  error?: string;
}
