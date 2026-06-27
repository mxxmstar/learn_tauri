/**
 * 远程文件管理模块前端类型定义。
 *
 * 这些类型需要尽量和 Rust 端返回结构保持一致，
 * 这样页面在开发时就能得到完整的类型提示，也更方便后续继续扩展。
 */

/**
 * SSH 连接请求参数。
 */
export interface SshConnectRequest {
  /** 远程主机地址，例如 192.168.66.117 */
  host: string;
  /** SSH 端口，默认通常为 22 */
  port: number;
  /** 登录用户名 */
  username: string;
  /** 登录密码 */
  password: string;
  /** 连接前经过用户确认的主机指纹 */
  expectedHostFingerprint?: string;
  /** 连接成功后默认进入的远程目录 */
  initialPath?: string;
}

/**
 * Tauri 命令统一返回结构。
 */
export interface SshCmdResult<T> {
  /** 命令是否执行成功 */
  success: boolean;
  /** 成功时返回的数据 */
  data?: T;
  /** 失败时的错误信息 */
  error?: string;
}

/**
 * SSH 会话基础信息。
 */
export interface SshSessionInfo {
  /** 后续所有远程文件操作都依赖这个会话 ID */
  sessionId: string;
  /** 远程主机地址 */
  host: string;
  /** SSH 端口 */
  port: number;
  /** 登录用户名 */
  username: string;
  /** 当前远程路径 */
  currentPath: string;
  /** 本次连接实际校验通过的主机指纹 */
  hostFingerprint: string;
}

/**
 * SSH 主机指纹探测请求。
 */
export interface SshHostProbeRequest {
  /** 远程主机地址 */
  host: string;
  /** SSH 端口 */
  port: number;
}

/**
 * SSH 主机指纹探测结果。
 */
export interface SshHostProbeResult {
  /** 远程主机地址 */
  host: string;
  /** SSH 端口 */
  port: number;
  /** 探测到的主机指纹 */
  fingerprint: string;
}

/**
 * 目录列表中的单个文件项。
 */
export interface RemoteFileEntry {
  /** 文件或目录名称 */
  name: string;
  /** 远程完整路径 */
  path: string;
  /** 是否为目录 */
  isDir: boolean;
  /** 是否为普通文件 */
  isFile: boolean;
  /** 是否为符号链接 */
  isSymlink: boolean;
  /** 文件类型描述 */
  fileType: string;
  /** 文件大小，单位字节 */
  size: number;
  /** 原始权限位 */
  permissions?: number;
  /** 八进制权限文本，例如 755 */
  permissionText?: string;
  /** 所属用户 ID */
  uid?: number;
  /** 所属用户组 ID */
  gid?: number;
  /** 修改时间 */
  modifiedAt?: string;
  /** 访问时间 */
  accessedAt?: string;
}

/**
 * 文件属性详情。
 */
export interface RemoteFileProperties {
  /** 文件或目录名称 */
  name: string;
  /** 远程完整路径 */
  path: string;
  /** 是否为目录 */
  isDir: boolean;
  /** 是否为普通文件 */
  isFile: boolean;
  /** 是否为符号链接 */
  isSymlink: boolean;
  /** 文件类型描述 */
  fileType: string;
  /** 文件大小，单位字节 */
  size: number;
  /** 原始权限位 */
  permissions?: number;
  /** 八进制权限文本 */
  permissionText?: string;
  /** 所属用户 ID */
  uid?: number;
  /** 所属用户组 ID */
  gid?: number;
  /** 修改时间 */
  modifiedAt?: string;
  /** 访问时间 */
  accessedAt?: string;
}

/**
 * 下载进度事件载荷。
 */
export interface DownloadProgress {
  /** 对应的 SSH 会话 ID */
  sessionId: string;
  /** 远程文件路径 */
  remotePath: string;
  /** 本地保存路径 */
  localPath: string;
  /** 已下载字节数 */
  downloadedBytes: number;
  /** 总字节数 */
  totalBytes: number;
  /** 进度比例，范围为 0 ~ 1 */
  progress: number;
  /** 当前阶段 */
  stage: "checking" | "downloading" | "saving" | "completed" | "error";
  /** 面向用户展示的提示信息 */
  message: string;
}

/**
 * 文件下载完成结果。
 */
export interface FileDownloadResult {
  /** 远程文件路径 */
  remotePath: string;
  /** 本地保存路径 */
  localPath: string;
  /** 文件大小 */
  fileSize: number;
  /** 整个下载耗时 */
  durationMs: number;
}

/**
 * 上传进度事件载荷。
 */
export interface UploadProgress {
  /** 对应的 SSH 会话 ID */
  sessionId: string;
  /** 本地源文件路径 */
  localPath: string;
  /** 远程目标文件路径 */
  remotePath: string;
  /** 已上传字节数 */
  uploadedBytes: number;
  /** 总字节数 */
  totalBytes: number;
  /** 进度比例，范围为 0 ~ 1 */
  progress: number;
  /** 当前阶段 */
  stage: "checking" | "uploading" | "saving" | "completed" | "error";
  /** 面向用户展示的提示信息 */
  message: string;
}

/**
 * 文件上传完成结果。
 */
export interface FileUploadResult {
  /** 本地源文件路径 */
  localPath: string;
  /** 远程目标文件路径 */
  remotePath: string;
  /** 文件大小 */
  fileSize: number;
  /** 整个上传耗时 */
  durationMs: number;
}

/**
 * 新建目录成功结果。
 */
export interface CreateDirectoryResult {
  /** 新建成功后的远程目录路径 */
  path: string;
}

/**
 * 重命名成功结果。
 */
export interface RenamePathResult {
  /** 原始远程路径 */
  oldPath: string;
  /** 重命名后的远程路径 */
  newPath: string;
}

/**
 * 删除成功结果。
 */
export interface DeletePathResult {
  /** 被删除的远程路径 */
  path: string;
  /** 删除目标是否为目录 */
  isDir: boolean;
}

/**
 * 后端建议的默认下载路径。
 */
export interface SuggestedDownloadPath {
  /** 推荐保存到本机的路径 */
  suggestedPath: string;
}

/**
 * 文本文件本地打开结果。
 *
 * 这里的“本地打开”策略是：
 * 1. 先把远程文件下载到本机缓存目录；
 * 2. 如果属于文本文件，再把文本内容返回给前端弹窗展示。
 */
export interface OpenFileResult {
  /** 远程文件路径 */
  remotePath: string;
  /** 本地缓存路径 */
  localPath: string;
  /** 文件大小 */
  fileSize: number;
  /** 是否识别为文本文件 */
  isText: boolean;
  /** 文本内容 */
  textContent?: string;
}
