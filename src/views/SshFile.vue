<script setup lang="ts">
/**
 * SSH 远程文件管理页面。
 *
 * 这是第五版页面实现，核心职责包括：
 * 1. 管理 SSH 连接表单与连接状态；
 * 2. 展示远程目录列表与面包屑路径；
 * 3. 支持右键菜单：下载、属性、文本打开、重命名、删除；
 * 4. 支持工具栏操作：刷新、返回上级、上传文件、上传目录、新建目录；
 * 5. 支持主机指纹确认、下载进度、上传进度、文本预览；
 * 6. 支持远程文本文件在线编辑并保存回远程主机；
 * 7. 支持远程目录递归下载与本地目录递归上传。
 */

import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { message } from "ant-design-vue";
import type { TableColumnsType } from "ant-design-vue";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import {
  createRemoteDirectory,
  deleteRemotePath,
  downloadRemoteDirectory,
  downloadRemoteFile,
  getRemoteFileProperties,
  getSuggestedDownloadPath,
  listRemoteDirectory,
  openRemoteTextFile,
  renameRemotePath,
  saveRemoteTextFile,
  sshConnect,
  sshDisconnect,
  sshProbeHost,
  uploadRemoteDirectory,
  uploadRemoteFile,
} from "../remote-files/api";
import type {
  DirectoryDownloadResult,
  DirectoryUploadResult,
  DownloadProgress,
  OpenFileResult,
  RemoteFileEntry,
  RemoteFileProperties,
  SshConnectRequest,
  SshSessionInfo,
  UploadProgress,
} from "../remote-files/types";

/**
 * 本地存储键名：用于缓存上一次连接时的非敏感表单字段。
 */
const REMOTE_FILE_FORM_STORAGE_KEY = "remote-file-form";

/**
 * 本地存储键名：用于缓存已经被用户信任的 SSH 主机指纹。
 */
const SSH_KNOWN_HOSTS_STORAGE_KEY = "remote-file-known-hosts";

/**
 * SSH 连接表单。
 *
 * 注意：
 * - 密码不会持久化保存；
 * - expectedHostFingerprint 仅作为本次连接时的内部状态使用。
 */
const connectForm = reactive<SshConnectRequest>({
  host: "",
  port: 22,
  username: "root",
  password: "",
  expectedHostFingerprint: "",
  initialPath: "/",
});

/**
 * 当前 SSH 会话信息。
 *
 * 为 null 时表示当前还未连接远程主机。
 */
const sessionInfo = ref<SshSessionInfo | null>(null);

/**
 * 当前正在浏览的远程目录路径。
 */
const currentPath = ref("/");

/**
 * 当前目录下的文件列表。
 */
const fileList = ref<RemoteFileEntry[]>([]);

/**
 * 页面级别加载状态。
 */
const connecting = ref(false);
const directoryLoading = ref(false);
const disconnecting = ref(false);
const uploading = ref(false);
const deleting = ref(false);

/**
 * 最近一次下载进度。
 */
const latestDownloadProgress = ref<DownloadProgress | null>(null);

/**
 * 最近一次上传进度。
 */
const latestUploadProgress = ref<UploadProgress | null>(null);

/**
 * 文件属性弹窗状态。
 */
const propertyModalOpen = ref(false);
const propertyLoading = ref(false);
const currentProperties = ref<RemoteFileProperties | null>(null);

/**
 * 文本预览弹窗状态。
 */
const previewModalOpen = ref(false);
const previewLoading = ref(false);
const previewSaving = ref(false);
const previewResult = ref<OpenFileResult | null>(null);
const previewDraftContent = ref("");

/**
 * 新建目录弹窗状态。
 */
const createDirectoryModalOpen = ref(false);
const createDirectorySubmitting = ref(false);
const createDirectoryForm = reactive({
  directoryName: "",
});

/**
 * 重命名弹窗状态。
 */
const renameModalOpen = ref(false);
const renameSubmitting = ref(false);
const renameForm = reactive({
  newName: "",
});
const renameTarget = ref<RemoteFileEntry | null>(null);

/**
 * 自定义右键菜单状态。
 */
const contextMenuState = reactive<{
  visible: boolean;
  x: number;
  y: number;
  record: RemoteFileEntry | null;
}>({
  visible: false,
  x: 0,
  y: 0,
  record: null,
});

/**
 * 文件表格列定义。
 */
const columns: TableColumnsType<RemoteFileEntry> = [
  {
    title: "名称",
    dataIndex: "name",
    key: "name",
    ellipsis: true,
    width: "38%",
  },
  {
    title: "类型",
    dataIndex: "fileType",
    key: "fileType",
    width: 120,
  },
  {
    title: "大小",
    dataIndex: "size",
    key: "size",
    width: 140,
    align: "right",
  },
  {
    title: "权限",
    dataIndex: "permissionText",
    key: "permissionText",
    width: 120,
    align: "center",
  },
  {
    title: "修改时间",
    dataIndex: "modifiedAt",
    key: "modifiedAt",
    width: 220,
  },
];

/**
 * 当前是否已经建立 SSH 会话。
 */
const isConnected = computed(() => Boolean(sessionInfo.value?.sessionId));

/**
 * 是否可以返回上一级目录。
 */
const canGoParent = computed(() => normalizeRemotePath(currentPath.value) !== "/");

/**
 * 根据当前路径计算面包屑分段。
 */
const breadcrumbSegments = computed(() => {
  const normalizedPath = normalizeRemotePath(currentPath.value);
  if (normalizedPath === "/") {
    return [{ label: "/", path: "/" }];
  }

  const result = [{ label: "/", path: "/" }];
  const pathSegments = normalizedPath.split("/").filter(Boolean);
  let accumulatedPath = "";

  for (const segment of pathSegments) {
    accumulatedPath += `/${segment}`;
    result.push({
      label: segment,
      path: accumulatedPath,
    });
  }

  return result;
});

/**
 * 右键菜单定位样式。
 */
const contextMenuStyle = computed(() => ({
  left: `${contextMenuState.x}px`,
  top: `${contextMenuState.y}px`,
}));

/**
 * 当前右键选中的文件是否支持文本预览。
 */
const canOpenAsText = computed(() => {
  const record = contextMenuState.record;
  if (!record || record.isDir) {
    return false;
  }
  return isTextPreviewCandidate(record.name);
});

/**
 * 当前预览内容是否已经被用户修改但尚未保存。
 */
const previewDirty = computed(() => {
  if (!previewResult.value) {
    return false;
  }
  return (previewResult.value.textContent || "") !== previewDraftContent.value;
});

/**
 * 当前是否允许执行“保存回远程”。
 */
const canSavePreview = computed(() => {
  return Boolean(
    sessionInfo.value?.sessionId &&
      previewResult.value?.isText &&
      !previewLoading.value &&
      !previewSaving.value &&
      previewDirty.value,
  );
});

/**
 * 页面挂载后恢复本地缓存并注册全局点击事件，
 * 用于点击空白处时关闭右键菜单。
 */
onMounted(() => {
  restoreConnectForm();
  document.addEventListener("click", closeContextMenu);
  document.addEventListener("scroll", closeContextMenu, true);
});

/**
 * 页面卸载时清理事件监听，避免内存泄漏。
 */
onBeforeUnmount(() => {
  document.removeEventListener("click", closeContextMenu);
  document.removeEventListener("scroll", closeContextMenu, true);
});

/**
 * 发起 SSH 连接。
 *
 * 第三版仍然保留第二版的严格指纹确认流程：
 * - 先探测主机指纹；
 * - 用户确认或比对本地缓存；
 * - 再把指纹带给后端做严格校验。
 */
async function handleConnect() {
  if (connecting.value) {
    return;
  }

  if (!connectForm.host.trim()) {
    message.warning("请输入远程主机地址");
    return;
  }

  if (!connectForm.username.trim()) {
    message.warning("请输入登录用户名");
    return;
  }

  connecting.value = true;
  closeContextMenu();

  try {
    const trustedFingerprint = await ensureTrustedHostFingerprint();
    const result = await sshConnect({
      host: connectForm.host.trim(),
      port: Number(connectForm.port) || 22,
      username: connectForm.username.trim(),
      password: connectForm.password,
      expectedHostFingerprint: trustedFingerprint,
      initialPath: connectForm.initialPath?.trim() || "/",
    });

    if (!result.success || !result.data) {
      throw new Error(result.error || "SSH 连接失败");
    }

    sessionInfo.value = result.data;
    currentPath.value = normalizeRemotePath(result.data.currentPath || "/");
    latestDownloadProgress.value = null;
    latestUploadProgress.value = null;

    persistConnectForm();
    await loadDirectory(currentPath.value);
    message.success(`已连接到 ${result.data.host}:${result.data.port}`);
  } catch (error) {
    message.error(extractErrorMessage(error, "SSH 连接失败"));
  } finally {
    connecting.value = false;
  }
}

/**
 * 断开当前 SSH 会话，并恢复页面到未连接状态。
 */
async function handleDisconnect() {
  if (!sessionInfo.value?.sessionId || disconnecting.value) {
    return;
  }

  disconnecting.value = true;
  closeContextMenu();

  try {
    const result = await sshDisconnect(sessionInfo.value.sessionId);
    if (!result.success) {
      throw new Error(result.error || "断开 SSH 连接失败");
    }

    resetConnectedState();
    message.success("SSH 连接已断开");
  } catch (error) {
    message.error(extractErrorMessage(error, "断开 SSH 连接失败"));
  } finally {
    disconnecting.value = false;
  }
}

/**
 * 加载指定远程目录。
 */
async function loadDirectory(path: string) {
  if (!sessionInfo.value?.sessionId) {
    return;
  }

  directoryLoading.value = true;
  closeContextMenu();

  try {
    const normalizedPath = normalizeRemotePath(path);
    const result = await listRemoteDirectory(sessionInfo.value.sessionId, normalizedPath);

    if (!result.success || !result.data) {
      throw new Error(result.error || `读取目录失败：${normalizedPath}`);
    }

    fileList.value = result.data;
    currentPath.value = normalizedPath;
  } catch (error) {
    message.error(extractErrorMessage(error, "读取远程目录失败"));
  } finally {
    directoryLoading.value = false;
  }
}

/**
 * 刷新当前目录。
 */
async function refreshCurrentDirectory() {
  await loadDirectory(currentPath.value);
}

/**
 * 返回上一级目录。
 */
async function goParentDirectory() {
  if (!canGoParent.value) {
    return;
  }
  await loadDirectory(getParentPath(currentPath.value));
}

/**
 * 点击面包屑后进入指定路径。
 */
async function goToPath(path: string) {
  if (!isConnected.value) {
    return;
  }
  await loadDirectory(path);
}

/**
 * 双击行时的默认行为：
 * - 目录：进入目录；
 * - 文本文件：本地打开并预览；
 * - 其他文件：提示用户使用下载。
 */
async function handleEntryDoubleClick(record: RemoteFileEntry) {
  closeContextMenu();

  if (record.isDir) {
    await loadDirectory(record.path);
    return;
  }

  if (isTextPreviewCandidate(record.name)) {
    await openTextPreview(record);
    return;
  }

  message.info("当前版本只支持文本类文件直接打开，其他文件请使用“下载到本地”");
}

/**
 * 打开自定义右键菜单。
 */
function openContextMenu(event: MouseEvent, record: RemoteFileEntry) {
  event.preventDefault();
  contextMenuState.visible = true;
  contextMenuState.x = event.clientX;
  contextMenuState.y = event.clientY;
  contextMenuState.record = record;
}

/**
 * 关闭右键菜单。
 */
function closeContextMenu() {
  contextMenuState.visible = false;
  contextMenuState.record = null;
}

/**
 * 从右键菜单直接进入目录。
 */
async function handleEnterDirectoryFromMenu() {
  const target = contextMenuState.record;
  closeContextMenu();

  if (!target?.isDir) {
    return;
  }

  await loadDirectory(target.path);
}

/**
 * 显示文件或目录属性。
 */
async function showProperties(record?: RemoteFileEntry) {
  const target = record || contextMenuState.record;
  closeContextMenu();

  if (!target || !sessionInfo.value?.sessionId) {
    return;
  }

  propertyModalOpen.value = true;
  propertyLoading.value = true;
  currentProperties.value = null;

  try {
    const result = await getRemoteFileProperties(sessionInfo.value.sessionId, target.path);
    if (!result.success || !result.data) {
      throw new Error(result.error || "读取文件属性失败");
    }
    currentProperties.value = result.data;
  } catch (error) {
    propertyModalOpen.value = false;
    message.error(extractErrorMessage(error, "读取文件属性失败"));
  } finally {
    propertyLoading.value = false;
  }
}

/**
 * 下载远程文件到本地。
 */
async function showDownloadDialog(record?: RemoteFileEntry) {
  const target = record || contextMenuState.record;
  closeContextMenu();

  if (!target || !sessionInfo.value?.sessionId) {
    return;
  }

  try {
    if (target.isDir) {
      const selectedDirectory = await open({
        title: "选择递归下载目录的本地保存位置",
        directory: true,
        multiple: false,
      });

      if (!selectedDirectory || Array.isArray(selectedDirectory)) {
        return;
      }

      latestDownloadProgress.value = null;

      const downloadResult = await downloadRemoteDirectory(
        sessionInfo.value.sessionId,
        target.path,
        selectedDirectory,
        (progress) => {
          latestDownloadProgress.value = progress;
        },
      );

      if (!downloadResult.success || !downloadResult.data) {
        throw new Error(downloadResult.error || "递归下载远程目录失败");
      }

      applyDirectoryDownloadSummary(downloadResult.data);
      message.success(
        `目录递归下载完成：${downloadResult.data.fileCount} 个文件，已保存到 ${downloadResult.data.localPath}`,
      );
      return;
    }

    const suggestedPathResult = await getSuggestedDownloadPath(target.name);
    const suggestedPath = suggestedPathResult.success && suggestedPathResult.data
      ? suggestedPathResult.data.suggestedPath
      : target.name;

    const selectedPath = await save({
      title: "选择本地保存位置",
      defaultPath: suggestedPath,
    });

    if (!selectedPath) {
      return;
    }

    const downloadResult = await downloadRemoteFile(
      sessionInfo.value.sessionId,
      target.path,
      selectedPath,
      (progress) => {
        latestDownloadProgress.value = progress;
      },
    );

    if (!downloadResult.success || !downloadResult.data) {
      throw new Error(downloadResult.error || "下载远程文件失败");
    }

    latestDownloadProgress.value = {
      sessionId: sessionInfo.value.sessionId,
      remotePath: downloadResult.data.remotePath,
      localPath: downloadResult.data.localPath,
      downloadedBytes: downloadResult.data.fileSize,
      totalBytes: downloadResult.data.fileSize,
      progress: 1,
      stage: "completed",
      message: "下载完成",
    };

    message.success(`文件已下载到：${downloadResult.data.localPath}`);
  } catch (error) {
    message.error(extractErrorMessage(error, "下载远程文件失败"));
  }
}

/**
 * 把目录递归下载的汇总结果同步到“最近下载任务”展示面板。
 */
function applyDirectoryDownloadSummary(result: DirectoryDownloadResult) {
  if (!sessionInfo.value?.sessionId) {
    return;
  }

  latestDownloadProgress.value = {
    sessionId: sessionInfo.value.sessionId,
    remotePath: result.remotePath,
    localPath: result.localPath,
    downloadedBytes: result.totalBytes,
    totalBytes: result.totalBytes,
    progress: 1,
    stage: "completed",
    message: `目录递归下载完成，共 ${result.fileCount} 个文件、${result.directoryCount} 个目录`,
  };
}

/**
 * 本地打开并预览简单文本文件。
 */
async function openTextPreview(record?: RemoteFileEntry) {
  const target = record || contextMenuState.record;
  closeContextMenu();

  if (!target || !sessionInfo.value?.sessionId) {
    return;
  }

  if (target.isDir) {
    message.info("目录不能作为文本文件打开");
    return;
  }

  previewModalOpen.value = true;
  previewLoading.value = true;
  previewSaving.value = false;
  previewResult.value = null;
  previewDraftContent.value = "";

  try {
    const result = await openRemoteTextFile(sessionInfo.value.sessionId, target.path);
    if (!result.success || !result.data) {
      throw new Error(result.error || "打开文本文件失败");
    }
    previewResult.value = result.data;
    previewDraftContent.value = result.data.textContent || "";
  } catch (error) {
    previewModalOpen.value = false;
    message.error(extractErrorMessage(error, "打开文本文件失败"));
  } finally {
    previewLoading.value = false;
  }
}

/**
 * 将编辑区内容恢复为刚打开文件时的原始内容。
 */
function resetPreviewDraft() {
  if (!previewResult.value) {
    return;
  }

  previewDraftContent.value = previewResult.value.textContent || "";
}

/**
 * 将文本编辑器中的内容保存回远程文件。
 */
async function savePreviewTextContent() {
  if (!sessionInfo.value?.sessionId || !previewResult.value || !canSavePreview.value) {
    return;
  }

  previewSaving.value = true;

  try {
    const result = await saveRemoteTextFile(
      sessionInfo.value.sessionId,
      previewResult.value.remotePath,
      previewDraftContent.value,
    );

    if (!result.success || !result.data) {
      throw new Error(result.error || "保存远程文本文件失败");
    }

    previewResult.value = {
      ...previewResult.value,
      textContent: previewDraftContent.value,
      fileSize: result.data.fileSize,
    };

    await refreshCurrentDirectory();
    message.success(
      `保存成功，已回写到远程文件（${formatFileSize(result.data.fileSize)}，耗时 ${result.data.durationMs} ms）`,
    );
  } catch (error) {
    message.error(extractErrorMessage(error, "保存远程文本文件失败"));
  } finally {
    previewSaving.value = false;
  }
}

/**
 * 打开“新建目录”弹窗。
 */
function openCreateDirectoryModal() {
  if (!sessionInfo.value?.sessionId) {
    return;
  }

  closeContextMenu();
  createDirectoryForm.directoryName = "";
  createDirectoryModalOpen.value = true;
}

/**
 * 提交“新建目录”操作。
 */
async function submitCreateDirectory() {
  if (!sessionInfo.value?.sessionId || createDirectorySubmitting.value) {
    return;
  }

  const directoryName = createDirectoryForm.directoryName.trim();
  if (!directoryName) {
    message.warning("请输入目录名称");
    return;
  }

  if (containsPathSeparator(directoryName)) {
    message.warning("目录名称中不能包含路径分隔符");
    return;
  }

  createDirectorySubmitting.value = true;

  try {
    const result = await createRemoteDirectory(
      sessionInfo.value.sessionId,
      currentPath.value,
      directoryName,
    );

    if (!result.success || !result.data) {
      throw new Error(result.error || "创建远程目录失败");
    }

    createDirectoryModalOpen.value = false;
    createDirectoryForm.directoryName = "";
    await refreshCurrentDirectory();
    message.success(`目录创建成功：${result.data.path}`);
  } catch (error) {
    message.error(extractErrorMessage(error, "创建远程目录失败"));
  } finally {
    createDirectorySubmitting.value = false;
  }
}

/**
 * 打开“重命名”弹窗。
 */
function openRenameModal(record?: RemoteFileEntry) {
  const target = record || contextMenuState.record;
  closeContextMenu();

  if (!target) {
    return;
  }

  renameTarget.value = target;
  renameForm.newName = target.name;
  renameModalOpen.value = true;
}

/**
 * 提交“重命名”操作。
 */
async function submitRename() {
  if (!sessionInfo.value?.sessionId || !renameTarget.value || renameSubmitting.value) {
    return;
  }

  const newName = renameForm.newName.trim();
  if (!newName) {
    message.warning("请输入新的名称");
    return;
  }

  if (containsPathSeparator(newName)) {
    message.warning("名称中不能包含路径分隔符");
    return;
  }

  if (newName === renameTarget.value.name) {
    renameModalOpen.value = false;
    return;
  }

  renameSubmitting.value = true;

  try {
    const result = await renameRemotePath(
      sessionInfo.value.sessionId,
      renameTarget.value.path,
      newName,
    );

    if (!result.success || !result.data) {
      throw new Error(result.error || "重命名远程文件失败");
    }

    renameModalOpen.value = false;
    renameTarget.value = null;
    renameForm.newName = "";
    await refreshCurrentDirectory();
    message.success(`已重命名为：${result.data.newPath}`);
  } catch (error) {
    message.error(extractErrorMessage(error, "重命名远程文件失败"));
  } finally {
    renameSubmitting.value = false;
  }
}

/**
 * 删除远程文件或目录。
 *
 * 为了安全起见，这里先弹确认框。
 * 第六版开始，目录删除会递归删除目录内全部内容。
 */
async function handleDeletePath(record?: RemoteFileEntry) {
  const target = record || contextMenuState.record;
  closeContextMenu();

  if (!target || !sessionInfo.value?.sessionId || deleting.value) {
    return;
  }

  const accepted = await confirm(
    target.isDir
      ? `确认递归删除目录：${target.path}\n\n注意：该操作会删除目录内所有子目录和文件，且无法恢复。`
      : `确认删除文件：${target.path}\n\n删除后将无法恢复。`,
    {
      title: "确认删除",
      kind: "warning",
      okLabel: "确认删除",
      cancelLabel: "取消",
    },
  );

  if (!accepted) {
    return;
  }

  deleting.value = true;

  try {
    const result = await deleteRemotePath(sessionInfo.value.sessionId, target.path);
    if (!result.success || !result.data) {
      throw new Error(result.error || "删除远程路径失败");
    }

    await refreshCurrentDirectory();
    message.success(result.data.isDir ? "目录递归删除成功" : "文件删除成功");
  } catch (error) {
    message.error(extractErrorMessage(error, "删除远程路径失败"));
  } finally {
    deleting.value = false;
  }
}

/**
 * 选择本地文件并上传到当前远程目录。
 */
async function handleUploadFile() {
  if (!sessionInfo.value?.sessionId || uploading.value) {
    return;
  }

  try {
    const selectedFile = await open({
      title: "选择要上传的本地文件",
      directory: false,
      multiple: false,
    });

    if (!selectedFile || Array.isArray(selectedFile)) {
      return;
    }

    uploading.value = true;
    latestUploadProgress.value = null;

    const result = await uploadRemoteFile(
      sessionInfo.value.sessionId,
      selectedFile,
      currentPath.value,
      (progress) => {
        latestUploadProgress.value = progress;
      },
    );

    if (!result.success || !result.data) {
      throw new Error(result.error || "上传本地文件失败");
    }

    latestUploadProgress.value = {
      sessionId: sessionInfo.value.sessionId,
      localPath: result.data.localPath,
      remotePath: result.data.remotePath,
      uploadedBytes: result.data.fileSize,
      totalBytes: result.data.fileSize,
      progress: 1,
      stage: "completed",
      message: "上传完成",
    };

    await refreshCurrentDirectory();
    message.success(`文件已上传到：${result.data.remotePath}`);
  } catch (error) {
    message.error(extractErrorMessage(error, "上传本地文件失败"));
  } finally {
    uploading.value = false;
  }
}

/**
 * 把目录递归上传的汇总结果同步到“最近上传任务”展示面板。
 */
function applyDirectoryUploadSummary(result: DirectoryUploadResult) {
  if (!sessionInfo.value?.sessionId) {
    return;
  }

  latestUploadProgress.value = {
    sessionId: sessionInfo.value.sessionId,
    localPath: result.localPath,
    remotePath: result.remotePath,
    uploadedBytes: result.totalBytes,
    totalBytes: result.totalBytes,
    progress: 1,
    stage: "completed",
    message: `目录递归上传完成，共 ${result.fileCount} 个文件、${result.directoryCount} 个目录`,
  };
}

/**
 * 选择本地目录并递归上传到当前远程目录。
 */
async function handleUploadDirectory() {
  if (!sessionInfo.value?.sessionId || uploading.value) {
    return;
  }

  try {
    const selectedDirectory = await open({
      title: "选择要递归上传的本地目录",
      directory: true,
      multiple: false,
    });

    if (!selectedDirectory || Array.isArray(selectedDirectory)) {
      return;
    }

    uploading.value = true;
    latestUploadProgress.value = null;

    const result = await uploadRemoteDirectory(
      sessionInfo.value.sessionId,
      selectedDirectory,
      currentPath.value,
      (progress) => {
        latestUploadProgress.value = progress;
      },
    );

    if (!result.success || !result.data) {
      throw new Error(result.error || "递归上传本地目录失败");
    }

    applyDirectoryUploadSummary(result.data);
    await refreshCurrentDirectory();
    message.success(
      `目录递归上传完成：${result.data.fileCount} 个文件，已上传到 ${result.data.remotePath}`,
    );
  } catch (error) {
    message.error(extractErrorMessage(error, "递归上传本地目录失败"));
  } finally {
    uploading.value = false;
  }
}

/**
 * 确保当前主机指纹已经被用户确认。
 */
async function ensureTrustedHostFingerprint() {
  const host = connectForm.host.trim();
  const port = Number(connectForm.port) || 22;

  const probeResult = await sshProbeHost({ host, port });
  if (!probeResult.success || !probeResult.data) {
    throw new Error(probeResult.error || "无法探测远程主机指纹");
  }

  const probedFingerprint = probeResult.data.fingerprint;
  const knownHosts = loadKnownHostsFromStorage();
  const cacheKey = buildKnownHostKey(host, port);
  const storedFingerprint = knownHosts[cacheKey];

  /**
   * 首次连接：要求用户显式确认。
   */
  if (!storedFingerprint) {
    const accepted = await confirm(
      `首次连接到 ${host}:${port}\n\n探测到的主机指纹：\n${probedFingerprint}\n\n请确认这就是你要连接的远程虚拟机。`,
      {
        title: "确认 SSH 主机指纹",
        kind: "warning",
        okLabel: "信任并连接",
        cancelLabel: "取消",
      },
    );

    if (!accepted) {
      throw new Error("用户取消了主机指纹确认");
    }

    saveKnownHostToStorage(host, port, probedFingerprint);
    connectForm.expectedHostFingerprint = probedFingerprint;
    return probedFingerprint;
  }

  /**
   * 已信任但发现指纹变更：必须再次确认。
   */
  if (storedFingerprint !== probedFingerprint) {
    const accepted = await confirm(
      `主机 ${host}:${port} 的指纹发生了变化。\n\n旧指纹：\n${storedFingerprint}\n\n新指纹：\n${probedFingerprint}\n\n如果这是预期变更，请重新信任；否则请取消并检查安全风险。`,
      {
        title: "SSH 主机指纹变更警告",
        kind: "warning",
        okLabel: "重新信任并连接",
        cancelLabel: "取消",
      },
    );

    if (!accepted) {
      throw new Error("主机指纹发生变化，连接已取消");
    }

    saveKnownHostToStorage(host, port, probedFingerprint);
    connectForm.expectedHostFingerprint = probedFingerprint;
    return probedFingerprint;
  }

  connectForm.expectedHostFingerprint = storedFingerprint;
  return storedFingerprint;
}

/**
 * 把当前连接相关状态恢复到“未连接”。
 */
function resetConnectedState() {
  sessionInfo.value = null;
  currentPath.value = "/";
  fileList.value = [];
  latestDownloadProgress.value = null;
  latestUploadProgress.value = null;
  currentProperties.value = null;
  previewResult.value = null;
  previewDraftContent.value = "";
  propertyModalOpen.value = false;
  previewModalOpen.value = false;
  previewSaving.value = false;
  createDirectoryModalOpen.value = false;
  renameModalOpen.value = false;
  renameTarget.value = null;
  createDirectoryForm.directoryName = "";
  renameForm.newName = "";
  closeContextMenu();
}

/**
 * 恢复本地缓存中的连接表单。
 *
 * 出于安全原因，不恢复密码。
 */
function restoreConnectForm() {
  try {
    const raw = localStorage.getItem(REMOTE_FILE_FORM_STORAGE_KEY);
    if (!raw) {
      return;
    }

    const parsed = JSON.parse(raw) as Partial<SshConnectRequest>;
    connectForm.host = parsed.host || "";
    connectForm.port = Number(parsed.port) || 22;
    connectForm.username = parsed.username || "root";
    connectForm.initialPath = parsed.initialPath || "/";
  } catch {
    /**
     * 如果本地缓存损坏，直接忽略，不阻塞页面使用。
     */
  }
}

/**
 * 保存连接表单中的非敏感字段。
 */
function persistConnectForm() {
  localStorage.setItem(
    REMOTE_FILE_FORM_STORAGE_KEY,
    JSON.stringify({
      host: connectForm.host,
      port: connectForm.port,
      username: connectForm.username,
      initialPath: connectForm.initialPath,
    }),
  );
}

/**
 * 从本地读取已信任主机表。
 */
function loadKnownHostsFromStorage(): Record<string, string> {
  try {
    const raw = localStorage.getItem(SSH_KNOWN_HOSTS_STORAGE_KEY);
    if (!raw) {
      return {};
    }

    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === "object") {
      return parsed as Record<string, string>;
    }

    return {};
  } catch {
    return {};
  }
}

/**
 * 把指定主机的可信指纹保存到本地。
 */
function saveKnownHostToStorage(host: string, port: number, fingerprint: string) {
  const knownHosts = loadKnownHostsFromStorage();
  knownHosts[buildKnownHostKey(host, port)] = fingerprint;
  localStorage.setItem(SSH_KNOWN_HOSTS_STORAGE_KEY, JSON.stringify(knownHosts));
}

/**
 * 构造主机缓存键。
 */
function buildKnownHostKey(host: string, port: number) {
  return `${host}:${port}`;
}

/**
 * 统一提取错误消息。
 */
function extractErrorMessage(error: unknown, fallbackMessage: string) {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return fallbackMessage;
}

/**
 * 统一规范远程路径格式。
 */
function normalizeRemotePath(path: string) {
  if (!path.trim()) {
    return "/";
  }
  return path.replace(/\\/g, "/").replace(/\/{2,}/g, "/");
}

/**
 * 计算父目录路径。
 */
function getParentPath(path: string) {
  const normalizedPath = normalizeRemotePath(path);
  if (normalizedPath === "/") {
    return "/";
  }

  const segments = normalizedPath.split("/").filter(Boolean);
  segments.pop();
  return segments.length === 0 ? "/" : `/${segments.join("/")}`;
}

/**
 * 判断名称中是否包含路径分隔符。
 *
 * 这类校验前后端都会做：
 * - 前端负责第一时间给用户友好提示；
 * - 后端负责最终兜底。
 */
function containsPathSeparator(value: string) {
  return value.includes("/") || value.includes("\\");
}

/**
 * 格式化文件大小。
 */
function formatFileSize(size: number) {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(2)} KB`;
  }
  if (size < 1024 * 1024 * 1024) {
    return `${(size / 1024 / 1024).toFixed(2)} MB`;
  }
  return `${(size / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/**
 * 格式化 ISO 时间字符串。
 */
function formatDateTime(value?: string) {
  if (!value) {
    return "--";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

/**
 * 缩略显示较长的指纹内容，避免工具栏占用过长空间。
 */
function formatFingerprintShort(value?: string) {
  if (!value) {
    return "--";
  }

  if (value.length <= 24) {
    return value;
  }

  return `${value.slice(0, 18)}...`;
}

/**
 * 判断文件名是否属于常见文本文件。
 */
function isTextPreviewCandidate(fileName: string) {
  const extension = fileName.includes(".")
    ? fileName.split(".").pop()?.toLowerCase() || ""
    : "";

  return [
    "txt",
    "log",
    "conf",
    "cfg",
    "ini",
    "json",
    "yaml",
    "yml",
    "xml",
    "toml",
    "md",
    "sh",
    "py",
    "rs",
    "ts",
    "js",
    "vue",
    "java",
    "c",
    "cpp",
    "h",
    "hpp",
    "csv",
  ].includes(extension);
}
</script>

<template>
  <div class="file-page">
    <section class="hero-panel">
      <div class="hero-copy">
        <p class="eyebrow">SSH Remote File Manager</p>
        <h2>通过 SSH 连接远程虚拟机，在本地浏览、下载、上传和维护文件</h2>
        <p class="hero-description">
          第五版在前面版本连接、目录浏览、文本编辑、下载上传和 SSH 独立入口的基础上，
          新增了目录递归下载与目录递归上传，让整个远程文件维护链路更加完整。
        </p>
      </div>

      <a-card class="connect-card" :bordered="false">
        <a-form layout="vertical">
          <a-row :gutter="12">
            <a-col :span="8">
              <a-form-item label="主机地址">
                <a-input
                  v-model:value="connectForm.host"
                  placeholder="例如：192.168.66.117"
                  :disabled="isConnected || connecting"
                />
              </a-form-item>
            </a-col>

            <a-col :span="4">
              <a-form-item label="端口">
                <a-input-number
                  v-model:value="connectForm.port"
                  class="full-width"
                  :min="1"
                  :max="65535"
                  :disabled="isConnected || connecting"
                />
              </a-form-item>
            </a-col>

            <a-col :span="6">
              <a-form-item label="用户名">
                <a-input
                  v-model:value="connectForm.username"
                  placeholder="例如：pyl"
                  :disabled="isConnected || connecting"
                />
              </a-form-item>
            </a-col>

            <a-col :span="6">
              <a-form-item label="初始目录">
                <a-input
                  v-model:value="connectForm.initialPath"
                  placeholder="/"
                  :disabled="isConnected || connecting"
                />
              </a-form-item>
            </a-col>
          </a-row>

          <a-row :gutter="12" align="bottom">
            <a-col :span="18">
              <a-form-item label="密码">
                <a-input-password
                  v-model:value="connectForm.password"
                  placeholder="输入 SSH 密码"
                  :disabled="isConnected || connecting"
                  @pressEnter="handleConnect"
                />
              </a-form-item>
            </a-col>

            <a-col :span="6">
              <div class="connect-actions">
                <a-button
                  type="primary"
                  class="action-button"
                  :loading="connecting"
                  :disabled="isConnected"
                  @click="handleConnect"
                >
                  连接
                </a-button>
                <a-button
                  class="action-button"
                  danger
                  :loading="disconnecting"
                  :disabled="!isConnected"
                  @click="handleDisconnect"
                >
                  断开
                </a-button>
              </div>
            </a-col>
          </a-row>
        </a-form>
      </a-card>
    </section>

    <section class="workspace-panel">
      <div class="panel-toolbar">
        <div class="toolbar-left">
          <a-button :disabled="!isConnected || !canGoParent" @click="goParentDirectory">
            上一级
          </a-button>

          <a-button :disabled="!isConnected" @click="refreshCurrentDirectory">
            刷新
          </a-button>

          <a-button type="dashed" :disabled="!isConnected" @click="openCreateDirectoryModal">
            新建目录
          </a-button>

          <a-button type="primary" :disabled="!isConnected" :loading="uploading" @click="handleUploadFile">
            上传文件
          </a-button>

          <a-button :disabled="!isConnected" :loading="uploading" @click="handleUploadDirectory">
            上传目录
          </a-button>
        </div>

        <div class="toolbar-right">
          <a-tag v-if="sessionInfo" color="geekblue">
            {{ sessionInfo.username }}@{{ sessionInfo.host }}:{{ sessionInfo.port }}
          </a-tag>

          <a-tag v-if="sessionInfo?.hostFingerprint" color="cyan">
            指纹 {{ formatFingerprintShort(sessionInfo.hostFingerprint) }}
          </a-tag>

          <a-tag v-else color="default">
            未连接
          </a-tag>
        </div>
      </div>

      <div class="breadcrumb-panel">
        <span class="breadcrumb-label">当前路径</span>
        <div class="breadcrumb-list">
          <button
            v-for="segment in breadcrumbSegments"
            :key="segment.path"
            class="breadcrumb-chip"
            :disabled="!isConnected"
            @click="goToPath(segment.path)"
          >
            {{ segment.label }}
          </button>
        </div>
      </div>

      <a-table
        :columns="columns"
        :data-source="fileList"
        :loading="directoryLoading"
        :pagination="false"
        row-key="path"
        size="middle"
        class="file-table"
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'name'">
            <div
              class="name-cell"
              @dblclick="handleEntryDoubleClick(record)"
              @contextmenu.prevent="openContextMenu($event, record)"
            >
              <a-tag v-if="record.isDir" color="blue" class="type-badge">目录</a-tag>
              <a-tag v-else-if="record.isSymlink" color="gold" class="type-badge">链接</a-tag>
              <a-tag v-else color="default" class="type-badge">文件</a-tag>
              <span class="name-text">{{ record.name }}</span>
            </div>
          </template>

          <template v-else-if="column.key === 'fileType'">
            {{ record.fileType }}
          </template>

          <template v-else-if="column.key === 'size'">
            {{ record.isDir ? "--" : formatFileSize(record.size) }}
          </template>

          <template v-else-if="column.key === 'permissionText'">
            {{ record.permissionText || "--" }}
          </template>

          <template v-else-if="column.key === 'modifiedAt'">
            {{ formatDateTime(record.modifiedAt) }}
          </template>
        </template>

        <template #emptyText>
          <div class="empty-state">
            <p>{{ isConnected ? "当前目录没有文件" : "连接远程虚拟机后，这里会显示文件列表" }}</p>
          </div>
        </template>
      </a-table>

      <div v-if="latestDownloadProgress" class="download-progress-panel">
        <div class="download-progress-header">
          <strong>最近下载任务</strong>
          <span>{{ latestDownloadProgress.message }}</span>
        </div>
        <div class="download-progress-path">
          {{ latestDownloadProgress.remotePath }}
        </div>
        <a-progress
          :percent="Math.round(latestDownloadProgress.progress * 100)"
          :status="latestDownloadProgress.stage === 'error' ? 'exception' : undefined"
        />
        <div class="download-progress-meta">
          <span>本地路径：{{ latestDownloadProgress.localPath }}</span>
          <span>
            {{ formatFileSize(latestDownloadProgress.downloadedBytes) }}
            /
            {{ formatFileSize(latestDownloadProgress.totalBytes) }}
          </span>
        </div>
      </div>

      <div v-if="latestUploadProgress" class="download-progress-panel upload-progress-panel">
        <div class="download-progress-header">
          <strong>最近上传任务</strong>
          <span>{{ latestUploadProgress.message }}</span>
        </div>
        <div class="download-progress-path">
          {{ latestUploadProgress.remotePath }}
        </div>
        <a-progress
          :percent="Math.round(latestUploadProgress.progress * 100)"
          :status="latestUploadProgress.stage === 'error' ? 'exception' : undefined"
          stroke-color="#1677ff"
        />
        <div class="download-progress-meta">
          <span>本地路径：{{ latestUploadProgress.localPath }}</span>
          <span>
            {{ formatFileSize(latestUploadProgress.uploadedBytes) }}
            /
            {{ formatFileSize(latestUploadProgress.totalBytes) }}
          </span>
        </div>
      </div>
    </section>

    <div
      v-if="contextMenuState.visible && contextMenuState.record"
      class="context-menu"
      :style="contextMenuStyle"
      @click.stop
    >
      <button
        v-if="contextMenuState.record.isDir"
        class="context-menu-item"
        @click="handleEnterDirectoryFromMenu"
      >
        进入目录
      </button>

      <button
        v-if="canOpenAsText"
        class="context-menu-item"
        @click="openTextPreview()"
      >
        打开文本
      </button>

      <button
        class="context-menu-item"
        @click="showDownloadDialog()"
      >
        下载到本地
      </button>

      <button
        class="context-menu-item"
        @click="openRenameModal()"
      >
        重命名
      </button>

      <button
        class="context-menu-item danger-item"
        :disabled="deleting"
        @click="handleDeletePath()"
      >
        删除
      </button>

      <button class="context-menu-item" @click="showProperties()">
        显示文件属性
      </button>
    </div>

    <a-modal
      v-model:open="propertyModalOpen"
      title="文件属性"
      width="720px"
      :footer="null"
    >
      <a-spin :spinning="propertyLoading">
        <a-descriptions v-if="currentProperties" :column="2" bordered size="small">
          <a-descriptions-item label="文件名">
            {{ currentProperties.name }}
          </a-descriptions-item>
          <a-descriptions-item label="类型">
            {{ currentProperties.fileType }}
          </a-descriptions-item>
          <a-descriptions-item label="路径" :span="2">
            {{ currentProperties.path }}
          </a-descriptions-item>
          <a-descriptions-item label="大小">
            {{ currentProperties.isDir ? "--" : formatFileSize(currentProperties.size) }}
          </a-descriptions-item>
          <a-descriptions-item label="权限">
            {{ currentProperties.permissionText || "--" }}
          </a-descriptions-item>
          <a-descriptions-item label="UID">
            {{ currentProperties.uid ?? "--" }}
          </a-descriptions-item>
          <a-descriptions-item label="GID">
            {{ currentProperties.gid ?? "--" }}
          </a-descriptions-item>
          <a-descriptions-item label="修改时间">
            {{ formatDateTime(currentProperties.modifiedAt) }}
          </a-descriptions-item>
          <a-descriptions-item label="访问时间">
            {{ formatDateTime(currentProperties.accessedAt) }}
          </a-descriptions-item>
        </a-descriptions>
      </a-spin>
    </a-modal>

    <a-modal
      v-model:open="previewModalOpen"
      title="文本文件本地打开与编辑"
      width="900px"
      :footer="null"
    >
      <a-spin :spinning="previewLoading">
        <div v-if="previewResult" class="preview-panel">
          <div class="preview-meta">
            <div>
              <span class="preview-meta-label">远程路径</span>
              <span>{{ previewResult.remotePath }}</span>
            </div>
            <div>
              <span class="preview-meta-label">本地缓存</span>
              <span>{{ previewResult.localPath }}</span>
            </div>
            <div>
              <span class="preview-meta-label">文件大小</span>
              <span>{{ formatFileSize(previewResult.fileSize) }}</span>
            </div>
          </div>

          <div class="preview-toolbar">
            <div class="preview-status" :class="{ dirty: previewDirty }">
              {{ previewDirty ? "当前内容已修改，尚未保存到远程主机" : "当前内容与远程文件保持一致" }}
            </div>
            <div class="preview-actions">
              <a-button :disabled="previewSaving || !previewDirty" @click="resetPreviewDraft">
                恢复原内容
              </a-button>
              <a-button
                type="primary"
                :loading="previewSaving"
                :disabled="!canSavePreview"
                @click="savePreviewTextContent"
              >
                保存回远程
              </a-button>
            </div>
          </div>

          <template v-if="previewResult.isText">
            <a-textarea
              v-model:value="previewDraftContent"
              class="preview-editor"
              :auto-size="{ minRows: 18, maxRows: 24 }"
              :disabled="previewSaving"
            />
            <p class="preview-hint">
              当前编辑器展示的是本地缓存内容，点击“保存回远程”后会直接覆盖远程同路径文件。
            </p>
          </template>
          <pre v-else class="preview-content">{{ previewResult.textContent || "" }}</pre>
        </div>
      </a-spin>
    </a-modal>

    <a-modal
      v-model:open="createDirectoryModalOpen"
      title="新建远程目录"
      ok-text="创建"
      cancel-text="取消"
      :confirm-loading="createDirectorySubmitting"
      @ok="submitCreateDirectory"
    >
      <a-form layout="vertical">
        <a-form-item label="目录名称">
          <a-input
            v-model:value="createDirectoryForm.directoryName"
            placeholder="请输入目录名称"
            @pressEnter="submitCreateDirectory"
          />
        </a-form-item>
        <div class="modal-tip">
          将在当前路径 <code>{{ currentPath }}</code> 下创建目录。
        </div>
      </a-form>
    </a-modal>

    <a-modal
      v-model:open="renameModalOpen"
      title="重命名远程文件"
      ok-text="确认"
      cancel-text="取消"
      :confirm-loading="renameSubmitting"
      @ok="submitRename"
    >
      <a-form layout="vertical">
        <a-form-item label="新的名称">
          <a-input
            v-model:value="renameForm.newName"
            placeholder="请输入新的名称"
            @pressEnter="submitRename"
          />
        </a-form-item>
        <div class="modal-tip">
          当前目标：<code>{{ renameTarget?.path || "--" }}</code>
        </div>
      </a-form>
    </a-modal>
  </div>
</template>

<style scoped>
.file-page {
  display: flex;
  flex-direction: column;
  gap: 20px;
  min-height: 100%;
  background:
    radial-gradient(circle at top right, rgba(22, 119, 255, 0.12), transparent 24%),
    linear-gradient(180deg, #f8fbff 0%, #f4f6f9 100%);
}

.hero-panel {
  display: grid;
  grid-template-columns: 1.1fr 1.4fr;
  gap: 18px;
  align-items: stretch;
}

.hero-copy {
  padding: 28px;
  border-radius: 20px;
  background: linear-gradient(145deg, #0f2747 0%, #143f73 100%);
  color: #f6fbff;
  box-shadow: 0 16px 36px rgba(15, 39, 71, 0.18);
}

.eyebrow {
  margin-bottom: 10px;
  color: rgba(246, 251, 255, 0.72);
  letter-spacing: 0.12em;
  font-size: 12px;
  text-transform: uppercase;
}

.hero-copy h2 {
  margin-bottom: 12px;
  font-size: 28px;
  line-height: 1.35;
}

.hero-description {
  margin: 0;
  color: rgba(246, 251, 255, 0.84);
  font-size: 15px;
  line-height: 1.8;
}

.connect-card {
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 12px 32px rgba(26, 48, 83, 0.08);
}

.full-width {
  width: 100%;
}

.connect-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-bottom: 24px;
}

.action-button {
  min-width: 98px;
}

.workspace-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 18px;
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 12px 32px rgba(26, 48, 83, 0.08);
}

.panel-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.breadcrumb-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px 16px;
  border-radius: 16px;
  background: linear-gradient(180deg, #f7fbff 0%, #eef4fb 100%);
  border: 1px solid rgba(20, 63, 115, 0.08);
}

.breadcrumb-label {
  color: #4b647d;
  font-size: 13px;
}

.breadcrumb-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.breadcrumb-chip {
  padding: 6px 12px;
  border: none;
  border-radius: 999px;
  background: #fff;
  color: #143f73;
  cursor: pointer;
  box-shadow: inset 0 0 0 1px rgba(20, 63, 115, 0.12);
  transition: all 0.2s ease;
}

.breadcrumb-chip:hover:not(:disabled) {
  background: #143f73;
  color: #fff;
}

.breadcrumb-chip:disabled {
  color: #99a8b9;
  cursor: not-allowed;
}

.file-table {
  border-radius: 16px;
  overflow: hidden;
}

.name-cell {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 36px;
  cursor: pointer;
}

.type-badge {
  min-width: 42px;
  text-align: center;
}

.name-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-state {
  padding: 40px 0;
  color: #7a8998;
}

.download-progress-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px;
  border-radius: 16px;
  background: linear-gradient(180deg, #fff8ec 0%, #fff3de 100%);
  border: 1px solid rgba(214, 142, 36, 0.18);
}

.upload-progress-panel {
  background: linear-gradient(180deg, #eef7ff 0%, #e1f0ff 100%);
  border: 1px solid rgba(22, 119, 255, 0.18);
}

.download-progress-header,
.download-progress-meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  color: #684b1b;
}

.download-progress-path {
  color: #805d21;
  word-break: break-all;
}

.context-menu {
  position: fixed;
  z-index: 3000;
  min-width: 210px;
  padding: 8px;
  border-radius: 14px;
  background: rgba(16, 23, 34, 0.95);
  box-shadow: 0 18px 36px rgba(0, 0, 0, 0.28);
  backdrop-filter: blur(12px);
}

.context-menu-item {
  width: 100%;
  padding: 10px 12px;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: #f2f6fb;
  text-align: left;
  cursor: pointer;
  transition: background 0.2s ease;
}

.context-menu-item:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.12);
}

.context-menu-item:disabled {
  color: rgba(242, 246, 251, 0.38);
  cursor: not-allowed;
}

.danger-item {
  color: #ffb3b0;
}

.danger-item:hover:not(:disabled) {
  background: rgba(255, 77, 79, 0.16);
}

.preview-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.preview-meta {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
  border-radius: 14px;
  background: #f7fafc;
}

.preview-meta > div {
  display: flex;
  gap: 10px;
  line-height: 1.7;
  word-break: break-all;
}

.preview-meta-label {
  min-width: 72px;
  color: #5d6c7a;
}

.preview-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.preview-status {
  color: #4f5f70;
  font-size: 13px;
}

.preview-status.dirty {
  color: #d46b08;
}

.preview-actions {
  display: flex;
  gap: 10px;
}

.preview-editor {
  font-family:
    "Cascadia Code", "JetBrains Mono", "Fira Code", "Microsoft YaHei UI", Consolas, monospace;
}

.preview-hint {
  margin: 0;
  color: #6b7b8c;
  font-size: 12px;
  line-height: 1.8;
}

.preview-content {
  max-height: 520px;
  overflow: auto;
  margin: 0;
  padding: 16px;
  border-radius: 16px;
  background: #0f1722;
  color: #edf3fb;
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
}

.modal-tip {
  color: #5f6f80;
  line-height: 1.7;
  word-break: break-all;
}

@media (max-width: 1200px) {
  .hero-panel {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 768px) {
  .panel-toolbar {
    flex-direction: column;
    align-items: stretch;
  }

  .toolbar-right {
    justify-content: flex-start;
  }

  .connect-actions {
    justify-content: flex-start;
    margin-bottom: 0;
  }
}
</style>
