/**
 * 远程文件管理模块前端 API 封装。
 *
 * 页面组件只关心“我要做什么”，
 * 这里负责把这些操作翻译成对 Tauri 后端命令的调用。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CreateDirectoryResult,
  DeletePathResult,
  DirectoryDownloadResult,
  DirectoryUploadResult,
  DownloadProgress,
  FileDownloadResult,
  FileUploadResult,
  OpenFileResult,
  RemoteFileEntry,
  RemoteFileProperties,
  RenamePathResult,
  SaveRemoteTextResult,
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
 * 当前版本只允许改“名称”，不做跨目录移动。
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
 * 第六版开始：
 * - 普通文件按单文件删除；
 * - 目录按整棵目录树递归删除。
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
 * 如果传入进度回调，函数会在命令执行期间监听后端进度事件。
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
 * 第五版新增：递归下载远程目录到本地目录。
 *
 * 注意这里的 `localDir` 表示“本地目标父目录”，
 * 后端会在它下面自动创建与远程目录同名的根目录。
 */
export async function downloadRemoteDirectory(
  sessionId: string,
  remotePath: string,
  localDir: string,
  onProgress?: (progress: DownloadProgress) => void,
): Promise<SshCmdResult<DirectoryDownloadResult>> {
  let unlisten: (() => void) | null = null;

  if (onProgress) {
    unlisten = await listen<DownloadProgress>("ssh-download-progress", (event) => {
      if (event.payload.sessionId === sessionId) {
        onProgress(event.payload);
      }
    });
  }

  try {
    return await invoke("sftp_download_directory", {
      sessionId,
      remotePath,
      localDir,
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
 * 第五版新增：递归上传本地目录到远程目录。
 *
 * 注意这里的 `remoteDir` 表示“远程父目录”，
 * 后端会在该目录下创建一个与本地目录同名的远程根目录。
 */
export async function uploadRemoteDirectory(
  sessionId: string,
  localPath: string,
  remoteDir: string,
  onProgress?: (progress: UploadProgress) => void,
): Promise<SshCmdResult<DirectoryUploadResult>> {
  let unlisten: (() => void) | null = null;

  if (onProgress) {
    unlisten = await listen<UploadProgress>("ssh-upload-progress", (event) => {
      if (event.payload.sessionId === sessionId) {
        onProgress(event.payload);
      }
    });
  }

  try {
    return await invoke("sftp_upload_directory", {
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

/**
 * 第四版新增：保存文本内容回远程文件。
 *
 * 这个接口通常会在“文本预览 / 编辑弹窗”里调用，
 * 把用户修改后的文本内容覆盖写回远程文件。
 */
export async function saveRemoteTextFile(
  sessionId: string,
  remotePath: string,
  textContent: string,
): Promise<SshCmdResult<SaveRemoteTextResult>> {
  return invoke("sftp_save_text_file", {
    sessionId,
    remotePath,
    textContent,
  });
}
