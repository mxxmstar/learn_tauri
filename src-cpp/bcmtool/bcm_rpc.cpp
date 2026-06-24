#include "bcm_rpc.h"
#include "rpc_connect.h"
#include "bcm_config.h"
#include "bcm_dmon.h"

BcmRPC::BcmRPC(const QString deviceIP, QObject *parent)
    : QObject(parent), deviceIP_(deviceIP)
{

}

BcmRPC::~BcmRPC()
{

}

// emit finished signal in the end
void BcmRPC::rebootFinished()
{
    int32_t retVal;
    BCM_HandleType hdl = 0;
    uint16_t port = 5555;
    uint32_t timeoutMs = 60000;

    QByteArray deviceIPByte = deviceIP_.toUtf8();
    const char *deviceIP = deviceIPByte.constData();

    retVal = RPC_Open(deviceIP, port, timeoutMs, &hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC connection failed, ret: 0x%1").arg(retVal, 2, 16, QChar('0')));
        return;
    }

    retVal = DMON_Reboot(hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC reboot failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
    } else {
        emit consolePrint(QString("Device restarted successfully."));
    }

    RPC_Close(hdl);
    emit consolePrint(QString("rpc finished"), PrintLevel::op);
    emit finished();
}

void BcmRPC::reboot()
{
    int32_t retVal;
    BCM_HandleType hdl = 0;
    uint16_t port = 5555;
    uint32_t timeoutMs = 60000;

    QByteArray deviceIPByte = deviceIP_.toUtf8();
    const char *deviceIP = deviceIPByte.constData();

    retVal = RPC_Open(deviceIP, port, timeoutMs, &hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC connection failed, ret: 0x%1").arg(retVal, 2, 16, QChar('0')));
        return;
    }

    retVal = DMON_Reboot(hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC reboot failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
    } else {
        emit consolePrint(QString("Device restarted successfully."));
    }

    RPC_Close(hdl);
}

void BcmRPC::showConfig(CONFIG_RpcMsg *readMsg)
{
    uint32_t index = 0;
    char name[32];
    char val[32];
    uint32_t len;

    emit consolePrint(" ");
    emit consolePrint("***GET CONFIG***");

    while (index < readMsg->len) {
        BCM_MemSet(name, 0, sizeof(name));
        BCM_MemSet(val, 0, sizeof(val));
        len = CONFIG_ExtractItem(&readMsg->ctx[index], name, val);
        if (len > 4) {
            emit consolePrint(QString("%1: %2").arg(name).arg(val));
            emit configPair(QString("%1:%2").arg(name).arg(val));
            index += len;
        } else {
            break;
        }
    }

    emit consolePrint(" ");
}

void BcmRPC::readConfig()
{
    int32_t retVal;
    BCM_HandleType hdl = 0;
    uint16_t port = 5555;
    uint32_t timeoutMs = 60000;
    CONFIG_RpcMsg readMsg;

    QByteArray deviceIPByte = deviceIP_.toUtf8();
    const char *deviceIP = deviceIPByte.constData();

    BCM_MemSet((void *)&readMsg, 0U, sizeof(CONFIG_RpcMsg));

    retVal = RPC_Open(deviceIP, port, timeoutMs, &hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC connection failed, ret: 0x%1").arg(retVal, 2, 16, QChar('0')));
        return;
    }

    retVal = CONFIG_RpcRead(hdl, &readMsg);
    if (retVal != 0) {
        emit consolePrint(QString("RPC read config failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
    } else {
        showConfig(&readMsg);
    }

    RPC_Close(hdl);
    emit consolePrint(QString("rpc finished"), PrintLevel::op);
    emit finished();
}

uint32_t BcmRPC::writeConfigMsg(const QString name, const QString val, uint8_t* msg)
{
    uint32_t index = 0;
    uint32_t header;
    int32_t retVal = BCM_ERR_OK;
    bool ok;

    if (name == "mirror mode") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_MEDIA_MIRROR, 1);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;
        msg[index] = (uint8_t)val.toUInt(&ok);
    } else if (name == "FPS") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_MEDIA_FPS, 1);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;
        msg[index] = (uint8_t)val.toUInt(&ok);
    } else if (name == "SOMEIP UDP port") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_MEDIA_SOMEIPUDPPORT, 2);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;
        msg[index] = (val.toUInt(&ok) >> 8) & 0xFF;
        msg[index + 1] = val.toUInt(&ok) & 0xFF;
    } else if (name == "SOMEIP RTP port") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_MEDIA_SOMEIPRTPPORT, 2);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;
        msg[index] = (val.toUInt(&ok) >> 8) & 0xFF;
        msg[index + 1] = val.toUInt(&ok) & 0xFF;
    } else  if (name == "DHCP") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_NETWORK_DHCP, 1);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;
        msg[index] = (uint8_t)val.toUInt(&ok);
    } else if (name == "IP") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_NETWORK_IP, 16);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;

        QByteArray valByte = val.toUtf8();
        const char *ip = valByte.constData();
        BCM_MemCpy(msg + index, ip, 16);
    } else if (name == "MAC") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_NETWORK_MAC, 20);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;

        QByteArray valByte = val.toUtf8();
        const char *mac = valByte.constData();
        BCM_MemCpy(msg + index, mac, 20);
    } else if (name == "AVTP DST MAC") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_AVTP_DSTMAC, 20);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;

        QByteArray valByte = val.toUtf8();
        const char *mac = valByte.constData();
        BCM_MemCpy(msg + index, mac, 20);
    } else if (name == "AVTP stream ID") {
        header = CONFIG_ITEM_HEADER_R(CONFIG_AVTP_STREAMID, 8);
        BCM_MemCpy(msg + index, &header, 4);
        index += 4;

        msg[index] = (val.toULongLong(&ok, 16) >> 56) & 0xFF;
        msg[index + 1] = (val.toULongLong(&ok, 16) >> 48) & 0xFF;
        msg[index + 2] = (val.toULongLong(&ok, 16) >> 40) & 0xFF;
        msg[index + 3] = (val.toULongLong(&ok, 16) >> 32) & 0xFF;
        msg[index + 4] = (val.toULongLong(&ok, 16) >> 24) & 0xFF;
        msg[index + 5] = (val.toULongLong(&ok, 16) >> 16) & 0xFF;
        msg[index + 6] = (val.toULongLong(&ok, 16) >> 8) & 0xFF;
        msg[index + 7] = val.toULongLong(&ok, 16) & 0xFF;
    } else {
        retVal = BCM_ERR_INVAL_PARAMS;
    }

    return retVal;
}

void BcmRPC::writeConfig(const QString name, const QString val)
{
    int32_t retVal;
    BCM_HandleType hdl = 0;
    uint16_t port = 5555;
    uint32_t timeoutMs = 60000;
    CONFIG_RpcMsg writeMsg;

    QByteArray deviceIPByte = deviceIP_.toUtf8();
    const char *deviceIP = deviceIPByte.constData();
    BCM_MemSet((void *)&writeMsg, 0U, sizeof(CONFIG_RpcMsg));
    retVal = writeConfigMsg(name, val, writeMsg.ctx);
    if (retVal != 0) {
        emit consolePrint(QString("RPC write config msg failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
        return;
    } else {
        QString hexStr;
        for (size_t i = 0; i < 20; i++) {
            hexStr += QString("%1").arg(writeMsg.ctx[i], 2, 16, QLatin1Char('0'));
            hexStr += " ";
        }
        emit consolePrint(hexStr);
    }

    retVal = RPC_Open(deviceIP, port, timeoutMs, &hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC connection failed, ret: 0x%1").arg(retVal, 2, 16, QChar('0')));
        return;
    }

    retVal = CONFIG_RpcWrite(hdl, &writeMsg);
    if (retVal != 0) {
        emit consolePrint(QString("RPC write config failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
    } else {
        emit consolePrint(QString("RPC write config successfully.")); 
    }

    RPC_Close(hdl);
    if (BCM_ERR_OK == retVal) {
        reboot();
        emit consolePrint(QString("save config"), PrintLevel::op);
    }
    emit consolePrint(QString("rpc finished"), PrintLevel::op);
    emit finished();
}

void BcmRPC::healthCheck(uint16_t pid)
{
    int32_t retVal;
    BCM_HandleType hdl = 0;
    uint32_t timeoutMs = 60000;
    IMGL_VersionType aVersion = {0};
    uint16_t port = 5555;

    QByteArray deviceIPByte = deviceIP_.toUtf8();
    const char *deviceIP = deviceIPByte.constData();

    retVal = RPC_Open(deviceIP, port, timeoutMs, &hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC connection failed, ret: 0x%1").arg(retVal, 2, 16, QChar('0')));
        return;
    }

    retVal = UPDATE_HealthCheck(hdl, pid, &aVersion);
    if (retVal != 0) {
        emit consolePrint(QString("RPC health check failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
    } else {
        emit consolePrint(QString("RPC health check, magic: 0x%1, major: %2, minor :%3")
                            .arg(aVersion.magic, 0, 16).arg(aVersion.major).arg(aVersion.minor));
    }

    RPC_Close(hdl);
    emit consolePrint(QString("rpc finished"), PrintLevel::op);
    emit finished();
}

void BcmRPC::getVersion()
{
    int32_t retVal;
    BCM_HandleType hdl = 0;
    uint16_t port = 5555;
    uint32_t timeoutMs = 60000;
    DMON_SwVersionMsgType aVersion = {0};

    QByteArray deviceIPByte = deviceIP_.toUtf8();
    const char *deviceIP = deviceIPByte.constData();

    retVal = RPC_Open(deviceIP, port, timeoutMs, &hdl);
    if (retVal != 0) {
        emit consolePrint(QString("RPC connection failed, ret: 0x%1").arg(retVal, 2, 16, QChar('0')));
        emit versionInfo(QString(""));
        return;
    }
    
    retVal = DMON_GetSwVersion(hdl, &aVersion);
    if (retVal != 0) {
        emit consolePrint(QString("RPC get version failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
        emit versionInfo(QString(""));
    } else {
        QString str = QString(aVersion.str);
        int index = str.indexOf("v1.");

        emit consolePrint("[Version] " + str.mid(index), PrintLevel::Info);
        emit versionInfo(str.mid(index));
    }

    RPC_Close(hdl);
    emit consolePrint(QString("rpc finished"), PrintLevel::op);
    emit finished();
}

void BcmRPC::fullInstall(uint32_t serverPort, const char* fileName, uint32_t fileSize, const char* ipv4_str_server)
{
    UPDATE_InstallCfgMsgType install = {0};
    uint16_t port = 5555;
    BCM_HandleType hdl = 0;
    QString outStr;
    uint32_t eraseSize = 0x1b0000;
    uint32_t flsId = 0;
    struct sockaddr_in address ={0};
    uint32_t rcvdSz = 0;
    uint32_t retVal;

    QByteArray deviceIPByte = deviceIP_.toUtf8();
    const char *deviceIP = deviceIPByte.constData();

    inet_pton(AF_INET, ipv4_str_server, &address.sin_addr);
    install.ipAddr = CPU_BEToNative32(address.sin_addr.s_addr);
    install.nvmChannel = IMGL_CHANNEL_ID_NVM_0 + flsId;
    install.fetchChannel = IMGL_CHANNEL_ID_RPC_FTP;
    install.nvmEraseSize = eraseSize;
    install.fileSize = fileSize;
    install.portNum = CPU_BEToNative16(serverPort);
    BCM_MemCpy(install.name, fileName, strlen(fileName) + 1);

    retVal = RPC_Open(deviceIP, port, 60000, &hdl);
    if (BCM_ERR_OK == retVal) {
        retVal = UPDATE_FullInstall(hdl, &install, &rcvdSz);
        if (BCM_ERR_OK == retVal) {
            emit consolePrint(QString("Th upgrade is successful, the size of the file received by client is %1.").arg(rcvdSz));
        } else {
            emit consolePrint(QString("Upgrade failed, ret: 0x%1 ").arg(retVal, 2, 16, QChar('0')));
        }
        RPC_Close(hdl);
    } else {
        emit consolePrint(QString("RPC connection failed, ret: 0x%1").arg(retVal, 2, 16, QChar('0')));
    }
    // reboot after upgrade successfully.
    if (BCM_ERR_OK == retVal) {
        reboot();
    }
    emit finished();
}
