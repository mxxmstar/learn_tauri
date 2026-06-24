#pragma execution_character_set("utf-8")
#include "bcm_flash.h"
#include "rpc_connect.h"


FileTransferServer::FileTransferServer(const QString fileName, const QByteArray fileBytes, const QString hostIP, QObject *parent)
    : QObject(parent), fileName_(fileName), fileBytes_(fileBytes), hostIP_(hostIP)
{
    fileName_.detach();
    fileBytes_.detach();
    hostIP_.detach();
}

void FileTransferServer::start()
{
    int32_t ret;
    int32_t fd0 = -1;
    int32_t fd1 = -1;
    const char* buffer;
    struct sockaddr_in address ={0};
    int32_t addrlen = sizeof(address);
    int fileSize = fileBytes_.size();

    QByteArray fileNameByte = fileName_.toUtf8();
    const char *fileName = fileNameByte.constData();
    QByteArray hostIPByte = hostIP_.toUtf8();
    const char *hostIP = hostIPByte.constData();

    fd0 = socket(AF_INET, SOCK_STREAM, 0);
    if (fd0 > 0) {
        address.sin_family = AF_INET;
        address.sin_addr.s_addr = INADDR_ANY;
        address.sin_port = htons(0);
        ret = bind(fd0, (struct sockaddr*)&address, addrlen);
    } 

    if (0 == ret) {
        ret = listen(fd0, 2);
    }

    if (0 == ret) {
        memset(&address, 0, addrlen);
        ret = getsockname(fd0, (struct sockaddr*)&address, &addrlen);
    }

    if (0 == ret) {
        emit consolePrint(QString("The server has been started, please wait..."));
        emit signal_progress(QString("升级服务已启动...（请耐心等待）"), 0, 0);
    } else {
        emit consolePrint(QString("The server startup failed."), PrintLevel::Error);
        emit finished();
        emit signal_progress(QString("启动服务器失败"), 0, -1);
        return;
    }
    emit ready(address.sin_port, fileName, fileSize, hostIP);

    fd1 = accept(fd0, (struct sockaddr*)&address, &addrlen);
    if (fd1 > 0) {
        int tmp = 0;
        int done = 0;
        QByteArray chunk;
        emit consolePrint(QString("The device is connected and update start."));
        do {
            tmp = qMin(256, fileSize - done);
            if (tmp > 0) {
                chunk = fileBytes_.mid(done, tmp);
                buffer = chunk.constData();
                ret = send(fd1, buffer, tmp, 0);
                done += tmp;
            }
            emit consolePrint(QString("Upgrade in progress %1").arg(done*100 / fileSize) + "%.");
            emit signal_progress(QString("升级中...（请勿执行其他操作）"), done*100 / fileSize, (done*100 / fileSize >= 100)?1:0);
        } while ((tmp > 0) && (ret >= 0));
    }

    if (fd1 > 0) {
        closesocket(fd1);
    }
    if (fd0 > 0) {
        closesocket(fd0);
    }

    emit signal_progress(QString("升级成功"), 100, 1);
    emit finished();
}

/**

    @trace #BRCM_SWARCH_UPDATE_HEALTH_CHECK_PROC
    @trace #BRCM_SWREQ_UPDATE

    @code{.unparsed}
    @endcode
*/
int32_t UPDATE_HealthCheck(BCM_HandleType aConnHdl, PTBL_IdType aPid, IMGL_VersionType *aVersion)
{
    int32_t retVal;

    if (NULL == aVersion) {
        retVal = BCM_ERR_INVAL_PARAMS;
    } else {
        uint32_t respLen = sizeof(RPC_MsgType);
        UPDATE_HealthCheckMsgType healthCheck;
        BCM_MemSet((uint8_t *)&healthCheck, 0, sizeof(UPDATE_HealthCheckMsgType));

        healthCheck.pid = CPU_NativeToLE16(aPid);
        retVal = RPC_SendRecv(aConnHdl, UPDATE_ID_HEALTH_CHECK, (const uint8_t *)&healthCheck,
                                      sizeof(UPDATE_HealthCheckMsgType),
                                      (uint8_t * const)&healthCheck, &respLen);
        if(BCM_ERR_OK == retVal) {
            BCM_MemCpy((uint8_t *)aVersion, (uint8_t *)&healthCheck.version, sizeof(IMGL_VersionType));
            aVersion->magic = CPU_LEToNative32(healthCheck.version.magic);
            aVersion->major = CPU_LEToNative32(healthCheck.version.major);
            aVersion->minor = CPU_LEToNative32(healthCheck.version.minor);
        }
    }

    return retVal;
}

/**

    @trace #BRCM_SWARCH_UPDATE_SAFE_INSTALL_PROC
    @trace #BRCM_SWARCH_UPDATE_FULL_INSTALL_PROC
    @trace #BRCM_SWARCH_UPDATE_RAW_INSTALL_PROC
    @trace #BRCM_SWREQ_UPDATE

    @code{.unparsed}
    @endcode
*/
uint32_t UPDATE_InstallHost(BCM_HandleType aConnHdl, UPDATE_InstallCfgMsgType *aInstallMsg,
    uint32_t *aRecvFileSize, BCM_MsgType aCmd)
{
    int32_t retVal;

    uint32_t respLen = sizeof(RPC_MsgType);
    UPDATE_InstallMsgType install;
    BCM_MemSet((uint8_t *)&install, 0, sizeof(UPDATE_InstallMsgType));

    install.cfg.nvmChannel = CPU_NativeToLE32(aInstallMsg->nvmChannel);
    install.cfg.fetchChannel = CPU_NativeToLE32(aInstallMsg->fetchChannel);
    install.cfg.nvmEraseSize = CPU_NativeToLE32(aInstallMsg->nvmEraseSize);
    install.cfg.fileSize = CPU_NativeToLE32(aInstallMsg->fileSize);
    install.cfg.ipAddr = CPU_NativeToLE32(aInstallMsg->ipAddr);
    install.cfg.portNum = CPU_NativeToLE32(aInstallMsg->portNum);
    install.recvFileSize = 0UL;
    BCM_MemCpy(&install.cfg.name[0UL], &aInstallMsg->name[0UL], UPDATE_MAX_FILENAME);
    retVal = RPC_SendRecv(aConnHdl, aCmd, (const uint8_t *)&install,
                                  sizeof(UPDATE_InstallMsgType),
                                  (uint8_t * const)&install, &respLen);
    if(BCM_ERR_OK == retVal) {
        *aRecvFileSize = CPU_LEToNative32(install.recvFileSize);
    }

    return retVal;
}

/**

    @trace #BRCM_SWARCH_UPDATE_FULL_INSTALL_PROC
    @trace #BRCM_SWREQ_UPDATE

    @code{.unparsed}
    @endcode
*/
int32_t UPDATE_FullInstall(BCM_HandleType aConnHdl, UPDATE_InstallCfgMsgType *aInstallMsg,
    uint32_t *aRecvFileSize)
{
    int32_t retVal;

    if ((NULL == aInstallMsg) ||
        (NULL == aRecvFileSize)) {
        retVal = BCM_ERR_INVAL_PARAMS;
    } else {
        retVal = UPDATE_InstallHost(aConnHdl, aInstallMsg, aRecvFileSize, UPDATE_ID_FULL_INSTALL);
    }

    return retVal;
}
