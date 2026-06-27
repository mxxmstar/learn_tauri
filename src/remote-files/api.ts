/**
 * 远程文件管理模块前端 API 封装。
 *
 * 页面组件只关心“我要做什么”，
 * 这里负责把页面意图翻译成对 Tauri 后端命令的调用。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CreateDirectoryResult,
  DeletePathResult,
  DownloadProgress,
  FileDownloadResult,
  FileUploadResult,
  OpenFileResult,
  RemoteFileEntry,
  RemoteFileProperties,
  RenamePathResult,
  SshCmdResult,
  SshConnectRequest,
  SshHostProbeRequest,
  SshHostProbeResult,
  SshSessionInfo,
  SuggestedDownloadPath,
  UploadProgress,
} from "./types";

/**
 * 建立 SSH 连接。
 */
export async function sshConnect(
  config: SshConnectRequest,
): Promise<SshCmdResult<SshSessionInfo>> {
  return invoke("ssh_connect", { config });
}

/**
 * 探测远程主机指纹。
 */
export async function sshProbeHost(
  config: SshHostProbeRequest,
): Promise<SshCmdResult<SshHostProbeResult>> {
  return invoke("ssh_probe_host", { config });
}

/**
 * 断开指定 SSH 会话。
 */
export async function sshDisconnect(
  sessionId: string,
): Promise<SshCmdResult<void>> {
  return invoke("ssh_disconnect", { sessionId });
}

/**
 * 列出远程目录内容。
 */
export async function listRemoteDirectory(
  sessionId: string,
  path: string,
): Promise<SshCmdResult<RemoteFileEntry[]>> {
  return invoke("sftp_list_directory", { sessionId, path });
}

/**
 * 读取远程文件或目录属性。
 */
export async function getRemoteFileProperties(
  sessionId: string,
  path: string,
): Promise<SshCmdResult<RemoteFileProperties>> {
  return invoke("sftp_get_properties", { sessionId, path });
}

/**
 * 创建远程目录。
 */
export async function createRemoteDirectory(
  sessionId: string,
  parentPath: string,
  directoryName: string,
): Promise<SshCmdResult<CreateDirectoryResult>> {
  return invoke("sftp_create_directory", {
    sessionId,
    parentPath,
    directoryName,
  });
}

/**
 * 重命名远程文件或目录。
 *
 * 这里仅允许修改“名称”，父目录保持不变，
 * 这样交互更简单，也更符合右键菜单的使用预期。
 */
export async function renameRemotePath(
  sessionId: string,
  sourcePath: string,
  newName: string,
): Promise<SshCmdResult<RenamePathResult>> {
  return invoke("sftp_rename_path", {
    sessionId,
    sourcePath,
    newName,
  });
}

/**
 * 删除远程文件或目录。
 *
 * 当前版本支持：
 * - 删除普通文件；
 * - 删除空目录。
 */
export async function deleteRemotePath(
  sessionId: string,
  path: string,
): Promise<SshCmdResult<DeletePathResult>> {
  return invoke("sftp_delete_path", {
    sessionId,
    path,
  });
}

/**
 * 获取默认下载路径建议。
 */
export async function getSuggestedDownloadPath(
  fileName: string,
): Promise<SshCmdResult<SuggestedDownloadPath>> {
  return invoke("sftp_suggest_download_path", { fileName });
}

/**
 * 下载远程文件到本地。
 *
 * 如果传入了进度回调，函数会在命令执行期间监听后端进度事件。
 */
export async function downloadRemoteFile(
  sessionId: string,
  remotePath: string,
  localPath: string,
  onProgress?: (progress: DownloadProgress) => void,
): Promise<SshCmdResult<FileDownloadResult>> {
  let unlisten: (() => void) | null = null;

  if (onProgress) {
    unlisten = await listen<DownloadProgress>("ssh-download-progress", (event) => {
      if (event.payload.sessionId === sessionId) {
        onProgress(event.payload);
      }
    });
  }

  try {
    return await invoke("sftp_download_file", {
      sessionId,
      remotePath,
      localPath,
    });
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

/**
 * 上传本地文件到当前远程目录。
 */
export async function uploadRemoteFile(
  sessionId: string,
  localPath: string,
  remoteDir: string,
  onProgress?: (progress: UploadProgress) => void,
): Promise<SshCmdResult<FileUploadResult>> {
  let unlisten: (() => void) | null = null;

  if (onProgress) {
    unlisten = await listen<UploadProgress>("ssh-upload-progress", (event) => {
      if (event.payload.sessionId === sessionId) {
        onProgress(event.payload);
      }
    });
  }

  try {
    return await invoke("sftp_upload_file", {
      sessionId,
      localPath,
      remoteDir,
    });
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

/**
 * 本地打开简单文本文件。
 */
export async function openRemoteTextFile(
  sessionId: string,
  remotePath: string,
): Promise<SshCmdResult<OpenFileResult>> {
  return invoke("sftp_open_text_file", { sessionId, remotePath });
}
